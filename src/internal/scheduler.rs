use crate::{
    internal::{backfill, bot, crawler, crawler::quotes, reminder},
    logging,
};
use chrono::Local;
use clokwerk::{AsyncScheduler, Interval, Job, TimeUnits};
use std::{env, time::Duration};

/// 啟動排程
pub async fn start() {
    let mut scheduler = AsyncScheduler::new();

    // Helper function to log success or error messages
    async fn log_result(action: &str, result: Result<(), anyhow::Error>) {
        match result {
            Ok(_) => {
                logging::info_file_async(format!("{} executed successfully.", action));
            }
            Err(why) => {
                logging::error_file_async(format!("Failed to {} because {:?}", action, why));
            }
        }
    }

    // Constants for logging messages
    const BACKFILL_FINANCIAL_STATEMENT: &str = "backfill::financial_statement::execute";
    const BACKFILL_NET_ASSET_VALUE_EMERGING: &str =
        "backfill::net_asset_value_per_share::emerging::execute";
    const BACKFILL_NET_ASSET_VALUE_ZERO_VALUE: &str =
        "backfill::net_asset_value_per_share::zero_value::execute";
    const BACKFILL_INTERNATIONAL_SECURITIES_IDENTIFICATION_NUMBER: &str =
        "backfill::international_securities_identification_number::execute";
    const BACKFILL_DELISTED_COMPANY: &str = "backfill::delisted_company::execute";
    const BACKFILL_REVENUE: &str = "backfill::revenue::execute";
    const BACKFILL_TAIWAN_CAPITALIZATION_WEIGHTED_STOCK_INDEX: &str =
        "backfill::taiwan_capitalization_weighted_stock_index::execute";
    const QUOTES_LISTED: &str = "quotes::listed::execute";

    scheduler
        .every(Interval::Days(1))
        .at("01:00:00")
        .run(|| async {
            //將未有上季度財報的股票，到雅虎財經下載後回寫到 financial_statement 表
            log_result(
                BACKFILL_FINANCIAL_STATEMENT,
                backfill::financial_statement::execute().await,
            )
            .await;
            //更新興櫃股票的每股淨值
            log_result(
                BACKFILL_NET_ASSET_VALUE_EMERGING,
                backfill::net_asset_value_per_share::emerging::execute().await,
            )
            .await;
        });

    scheduler
        .every(Interval::Days(1))
        .at("03:00:00")
        .run(|| async {
            //從yahoo取得每股淨值數據，將未下市但每股淨值為零的股票更新其數據
            log_result(
                BACKFILL_NET_ASSET_VALUE_ZERO_VALUE,
                backfill::net_asset_value_per_share::zero_value::execute().await,
            )
            .await;
        });

    //每日五點更新台股台股國際證券識別碼
    scheduler
        .every(Interval::Days(1))
        .at("5:00:00")
        .run(|| async {
            //取得台股國際證券識別碼
            log_result(
                BACKFILL_INTERNATIONAL_SECURITIES_IDENTIFICATION_NUMBER,
                backfill::international_securities_identification_number::execute().await,
            )
            .await;
            //更新下市的股票
            log_result(
                BACKFILL_DELISTED_COMPANY,
                backfill::delisted_company::execute().await,
            )
            .await;
            //取得台股的營收
            log_result(BACKFILL_REVENUE, backfill::revenue::execute().await).await;
        });

    //每日上午八點
    scheduler
        .every(Interval::Days(1))
        .at("08:00:00")
        .run(|| async {
            //提醒本日除權息的股票
            reminder::ex_dividend::execute().await;
        });

    //每日下午三點
    scheduler
        .every(Interval::Days(1))
        .at("15:00:00")
        .run(|| async {
            //更新台股收盤指數
            log_result(
                BACKFILL_TAIWAN_CAPITALIZATION_WEIGHTED_STOCK_INDEX,
                backfill::taiwan_capitalization_weighted_stock_index::execute().await,
            )
            .await;

            //取得上市收盤報價數據
            log_result(QUOTES_LISTED, quotes::listed::visit(Local::now()).await).await;
        });


    scheduler.every(60.seconds()).run(|| async {
        crawler::free_dns::update().await;
    });

    tokio::spawn(async move {
        loop {
            scheduler.run_pending().await;
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    });

    let msg = format!(
        "StockCrawler 已啟動\r\nRust OS/Arch: {}/{}\r\n",
        env::consts::OS,
        env::consts::ARCH
    );

    let _ = bot::telegram::send_to_allowed(&msg).await;
}
