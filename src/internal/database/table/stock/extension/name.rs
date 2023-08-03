use anyhow::*;
use chrono::{
    Datelike,
    NaiveDate
};
use sqlx::FromRow;

use crate::internal::database;

#[derive(FromRow, Debug)]
pub struct SymbolAndName {
    pub stock_symbol: String,
    pub name: String,
}

/// 取得指定日期為除息權日的股票
pub async fn fetch_stocks_with_dividends_on_date(date: NaiveDate) -> Result<Vec<SymbolAndName>> {
    let sql = r#"
SELECT
    s.stock_symbol,
    s."Name" AS name
FROM
    stocks AS s 
INNER JOIN
     dividend AS d ON s.stock_symbol = d.security_code
WHERE
    d."year" = $1
    AND (d."ex-dividend_date1" = $2 OR d."ex-dividend_date2" = $2);
"#;

    let year = date.year();
    let date_str = date.format("%Y-%m-%d").to_string();

    sqlx::query_as::<_, SymbolAndName>(sql)
        .bind(year)
        .bind(&date_str)
        .fetch_all(database::get_connection())
        .await
        .context("Failed to fetch_stocks_with_dividends_on_date from database")
}

#[cfg(test)]
mod tests {
    use core::result::Result::Ok;

    use chrono::{Local, TimeZone};

    use crate::internal::logging;

    use super::*;

    #[tokio::test]
    async fn test_fetch_stocks_with_specified_ex_dividend_date() {
        dotenv::dotenv().ok();
        logging::debug_file_async("開始 fetch_stocks_with_specified_ex_dividend_date".to_string());

        let ex_date = Local.with_ymd_and_hms(2023, 4, 20, 0, 0, 0).unwrap();
        let d = ex_date.date_naive();
        match fetch_stocks_with_dividends_on_date(d).await {
            Ok(cd) => {
                dbg!(&cd);
                logging::debug_file_async(format!("stock: {:?}", cd));
            }
            Err(why) => {
                logging::debug_file_async(format!("Failed to execute because {:?}", why));
            }
        }

        logging::debug_file_async("結束 fetch_stocks_with_specified_ex_dividend_date".to_string());
    }
}