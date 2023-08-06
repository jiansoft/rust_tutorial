use std::{env, future::Future, result::Result::Ok};

use anyhow::*;
use chrono::{Local, NaiveDate};
use tokio_cron_scheduler::{Job, JobScheduler};

use crate::internal::{
    backfill, bot, calculation, crawler, database::table::daily_quote, logging, reminder,
};
use crate::internal::cache::{TTL, TtlCacheInner};
use crate::internal::database::table::estimate::Estimate;
use crate::internal::database::table::last_daily_quotes;
use crate::internal::database::table::yield_rank::YieldRank;

/// 啟動排程
pub async fn start() -> Result<()> {
    run_cron().await.map_err(|why| {
        logging::error_file_async(format!("Failed to run_cron because {:?}", why));
        why
    })?;

    let msg = format!(
        "StockCrawler 已啟動\r\nRust OS/Arch: {}/{}\r\n",
        env::consts::OS,
        env::consts::ARCH
    );

    bot::telegram::send(&msg).await.map_err(|err| {
        logging::error_file_async(format!("Failed to send telegram message because {:?}", err));
        err
    })
}

pub async fn run_cron() -> Result<()> {
    let sched = JobScheduler::new().await?;
    //                 sec  min   hour   day of month   month   day of week   year
    //let expression = "0   30   9,12,15     1,15       May-Aug  Mon,Wed,Fri  2018/2";
    // UTC 時間

    let jobs = vec![
        // 01:00 更新台股季度財報
        create_job(
            "0 0 17 * * *",
            backfill::financial_statement::quarter::execute,
        ),
        // 01:00 更新興櫃股票的每股淨值
        create_job(
            "0 0 17 * * *",
            backfill::net_asset_value_per_share::emerging::execute,
        ),
        // 05:00 更新台股年度財報
        create_job(
            "0 0 21 * * *",
            backfill::financial_statement::annual::execute,
        ),
        // 05:00 從yahoo取得每股淨值數據，將未下市但每股淨值為零的股票更新其數據
        create_job(
            "0 0 21 * * *",
            backfill::net_asset_value_per_share::zero_value::execute,
        ),
        // 05:00 取得台股的營收
        create_job("0 0 21 * * *", backfill::revenue::execute),
        // 05:00 更新台股國際證券識別碼
        create_job("0 0 21 * * *", backfill::isin::execute),
        // 05:00 更新下市的股票
        create_job("0 0 21 * * *", backfill::delisted_company::execute),
        // 05:00 更新股票權值佔比
        create_job("0 0 21 * * *", backfill::stock_weight::execute),
        // 08:00 提醒本日除權息的股票
        create_job("0 0 0 * * *", reminder::ex_dividend::execute),
        // 15:00 更新台股收盤指數
        create_job("0 0 7 * * *", backfill::taiwan_stock_index::execute),
        // 15:01 取得收盤報價數據
        create_job("0 0 7 * * *", || async {
            let current_date: NaiveDate = Local::now().date_naive();
            //抓取上市櫃公司每日收盤資訊
            backfill::quote::execute().await?;
            let daily_quote_count = daily_quote::fetch_count_by_date(current_date).await?;
            if daily_quote_count == 0 {
                return Ok(());
            }

            // 補上當日缺少的每日收盤數據
            let lack_daily_quotes_count =
                daily_quote::makeup_for_the_lack_daily_quotes(current_date).await?;
            logging::info_file_async(format!(
                "補上當日缺少的每日收盤數據結束:{:#?}",
                lack_daily_quotes_count
            ));

            // 計算均線
            calculation::daily_quotes::calculate_moving_average(current_date).await?;
            logging::info_file_async("計算均線結束".to_string());

            // 重建 last_daily_quotes 表內的數據
            last_daily_quotes::LastDailyQuotes::rebuild().await?;
            logging::info_file_async("重建 last_daily_quotes 表內的數據結束".to_string());

            // 計算便宜、合理、昂貴價的估算
            Estimate::insert(current_date).await?;
            logging::info_file_async("計算便宜、合理、昂貴價的估算結束".to_string());

            // 重建指定日期的 yield_rank 表內的數據
            YieldRank::upsert(current_date).await?;
            logging::info_file_async("重建 yield_rank 表內的數據結束".to_string());

            // 計算帳戶內市值
            calculation::money_history::calculate_money_history(current_date).await?;
            logging::info_file_async("計算帳戶內市值結束".to_string());

            //清除記憶與Redis內所有的快取
            TTL.clear();

            Ok(())
        }),
        // 21:00 資料庫內尚未有年度配息數據的股票取出後向第三方查詢後更新回資料庫
        create_job("0 0 13 * * *", backfill::dividend::execute),
        // 每分鐘更新一次ddns的ip
        create_job("0 * * * * *", crawler::free_dns::execute),
    ];

    for job in jobs.into_iter().flatten() {
        sched.add(job).await?;
    }

    sched.start().await?;

    Ok(())
}

fn create_job<F, Fut>(cron_expr: &'static str, task: F) -> Result<Job>
where
    F: Fn() -> Fut + Clone + Send + Sync + 'static,
    Fut: Future<Output = Result<(), Error>> + Send,
{
    Ok(Job::new_async(cron_expr, move |_uuid, _l| {
        let task = task.clone();
        Box::pin(async move {
            if let Err(why) = task().await {
                logging::error_file_async(format!("Failed to execute task because {:?}", why));
            }
        })
    })?)
}

#[cfg(test)]
mod tests {
    use tokio::time::{Duration, sleep};

    // 注意這個慣用法：在 tests 模組中，從外部範疇匯入所有名字。
    use super::*;

    async fn run() -> Result<()> {
        let sched = JobScheduler::new().await?;
        let every_minute = Job::new_async("* * * * * *", |_uuid, _l| {
            Box::pin(async move {
                println!("_uuid {:?} now: {:?}", _uuid, chrono::Local::now());
                dbg!("_uuid {:?} now: {:?}", _uuid, chrono::Local::now());
                logging::debug_file_async(format!(
                    "_uuid {:?} now: {:?}",
                    _uuid,
                    chrono::Local::now()
                ));
            })
        })?;
        sched.add(every_minute).await?;

        sched.start().await?;

        Ok(())
    }

    #[tokio::test]
    async fn test_split() {
        dotenv::dotenv().ok();
        run().await.expect("TODO: panic message");
        sleep(Duration::from_secs(240)).await;
        //loop {}
        //println!("split: {:?}, elapsed time: {:?}", result, end);
    }
}
