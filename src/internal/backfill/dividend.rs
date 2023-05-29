use crate::internal::{
    crawler::{goodinfo, yahoo},
    database::table::{self, dividend},
    logging,
};
use anyhow::*;
use chrono::{Datelike, Local};
use core::result::Result::Ok;
use std::{thread, time::Duration};

/// 更新股利發送數據
/// 資料庫內尚未有年度配息數據的股票取出後向第三方查詢後更新回資料庫
pub async fn execute() -> Result<()> {
    //尚未有股利或多次配息
    let now = Local::now();
    let year = now.year();
    let without_or_multiple = processing_without_or_multiple(year);
    let yahoo = processing_with_unannounced_ex_dividend_dates(year);
    let (res_without_or_multiple, res_yahoo) = tokio::join!(without_or_multiple, yahoo);

    match res_without_or_multiple {
        Ok(_) => {
            logging::info_file_async(
                "processing_without_or_multiple executed successfully.".to_string(),
            );
        }
        Err(why) => {
            logging::error_file_async(format!(
                "Failed to process_without_or_multiple because {:?}",
                why
            ));
        }
    }

    match res_yahoo {
        Ok(_) => {
            logging::info_file_async(
                "processing_with_unannounced_ex_dividend_dates executed successfully.".to_string(),
            );
        }
        Err(why) => {
            logging::error_file_async(format!("Failed to process_yahoo because {:?}", why));
        }
    }

    Ok(())
}

/// Asynchronously processes the stocks that have no dividends or have issued multiple dividends.
///
/// This function fetches a list of stock symbols that either have no dividends or have issued multiple dividends
/// in the current year. It then visits the dividend information of each stock symbol using the `goodinfo::dividend::visit`
/// function. For each dividend found, if the dividend's year is not the current year, it skips to the next dividend.
/// Otherwise, it converts the dividend into a `table::dividend::Entity` and tries to upsert it.
///
/// If the upsert operation is successful, it logs the success and the entity that was upserted.
/// If the upsert operation fails, it logs the error.
///
/// Between each stock symbol processing, the function sleeps for 6 seconds to prevent too many requests to the
/// `goodinfo::dividend::visit` function.
///
/// Returns `Ok(())` if the function finishes processing all stock symbols. If any error occurs during the process,
/// it returns `Err(e)`, where `e` is the error.
///
/// # Errors
///
/// This function will return an error if:
/// - It fails to fetch the list of stock symbols.
/// - It fails to visit the dividend information of a stock symbol.
/// - It fails to upsert a dividend entity.
async fn processing_without_or_multiple(year: i32) -> Result<()> {
    //尚未有股利或多次配息
    let stock_symbols = dividend::fetch_without_or_multiple(year).await?;
    logging::info_file_async(format!("本次殖利率的採集需收集 {} 家", stock_symbols.len()));
    for stock_symbol in stock_symbols {
        let dividends = goodinfo::dividend::visit(&stock_symbol).await?;
        thread::sleep(Duration::from_secs(90));
        // 取成今年度的股利數據
        let dividend_details = match dividends.get(&year) {
            Some(details) => details,
            None => continue,
        };

        for dividend in dividend_details {
            if dividend.year != year {
                continue;
            }

            let entity = table::dividend::Dividend::from(dividend);
            match entity.upsert().await {
                Ok(_) => {
                    logging::info_file_async(format!(
                        "dividend upsert executed successfully. \r\n{:#?}",
                        entity
                    ));
                }
                Err(why) => {
                    logging::error_file_async(format!("Failed to upsert because {:?} ", why));
                }
            }
        }
    }

    Ok(())
}

async fn processing_with_unannounced_ex_dividend_dates(year: i32) -> Result<()> {
    //除息日 尚未公布
    let dividends = dividend::Dividend::fetch_unpublished_dividends_for_year(year).await?;
    logging::info_file_async(format!("本次除息日的採集需收集 {} 家", dividends.len()));
    for mut entity in dividends {
        let yahoo = match yahoo::dividend::visit(&entity.security_code).await {
            Ok(yahoo_dividend) => yahoo_dividend,
            Err(why) => {
                logging::error_file_async(format!(
                    "Failed to yahoo::dividend::visit because {:?} ",
                    why
                ));
                continue;
            }
        };

        // 取成今年度的股利數據
        let yahoo_dividend_details = match yahoo.dividend.get(&year) {
            Some(details) => details,
            None => continue,
        };

        let yahoo_dividend_detail = yahoo_dividend_details.iter().find(|detail| {
            detail.year_of_dividend == entity.year_of_dividend
                && detail.quarter == entity.quarter
                && (detail.ex_dividend_date1 != entity.ex_dividend_date1
                    || detail.ex_dividend_date2 != entity.ex_dividend_date2)
        });

        if let Some(yahoo_dividend_detail) = yahoo_dividend_detail {
            entity.ex_dividend_date1 = yahoo_dividend_detail.ex_dividend_date1.to_string();
            entity.ex_dividend_date2 = yahoo_dividend_detail.ex_dividend_date2.to_string();
            entity.payable_date1 = yahoo_dividend_detail.payable_date1.to_string();
            entity.payable_date2 = yahoo_dividend_detail.payable_date2.to_string();

            if let Err(why) = entity.update_dividend_date().await {
                logging::error_file_async(format!(
                    "Failed to update_dividend_date because {:?} ",
                    why
                ));
            } else {
                logging::info_file_async(format!(
                    "dividend update_dividend_date executed successfully. \r\n{:#?}",
                    entity
                ));
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::internal::cache::SHARE;
    // 注意這個慣用法：在 tests 模組中，從外部範疇匯入所有名字。
    use super::*;

    #[tokio::test]
    async fn test_processing_with_unannounced_ex_dividend_dates() {
        dotenv::dotenv().ok();
        SHARE.load().await;
        logging::debug_file_async("開始 processing_with_unannounced_ex_dividend_dates".to_string());

        match processing_with_unannounced_ex_dividend_dates(2023).await {
            Ok(_) => {
                logging::debug_file_async(
                    "processing_with_unannounced_ex_dividend_dates executed successfully."
                        .to_string(),
                );
            }
            Err(why) => {
                logging::debug_file_async(format!(
                    "Failed to processing_with_unannounced_ex_dividend_dates because {:?}",
                    why
                ));
            }
        }

        logging::debug_file_async("結束 processing_with_unannounced_ex_dividend_dates".to_string());
    }

    #[tokio::test]
    async fn test_processing_without_or_multiple() {
        dotenv::dotenv().ok();
        SHARE.load().await;
        logging::debug_file_async("開始 processing_without_or_multiple".to_string());

        match processing_without_or_multiple(2023).await {
            Ok(_) => {
                logging::debug_file_async(
                    "processing_without_or_multiple executed successfully.".to_string(),
                );
            }
            Err(why) => {
                logging::debug_file_async(format!(
                    "Failed to processing_without_or_multiple because {:?}",
                    why
                ));
            }
        }

        logging::debug_file_async("結束 processing_without_or_multiple".to_string());
    }
}