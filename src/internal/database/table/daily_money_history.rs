use anyhow::{anyhow, Result};
use chrono::{DateTime, Duration, Local, NaiveDate};
use sqlx::{Postgres, postgres::PgQueryResult, Transaction};

use crate::internal::database;

/// 每日市值變化歷史記錄
#[derive(sqlx::FromRow, Debug)]
pub struct DailyMoneyHistory {
    pub date: NaiveDate,
    pub created_at: DateTime<Local>,
    pub updated_at: DateTime<Local>,
    pub unice: f64,
    pub eddie: f64,
    pub sum: f64,
}

impl DailyMoneyHistory {
    pub async fn upsert(
        date: NaiveDate,
        tx: &mut Option<Transaction<'_, Postgres>>,
    ) -> Result<PgQueryResult> {
        let one_month_ago = date - Duration::days(30);
        let sql = format!(
            r#"
WITH daily_quotes AS (
	SELECT "SecurityCode", "ClosingPrice"
	FROM "DailyQuotes"
	WHERE "Serial" IN (
		SELECT MAX("Serial") AS serial
		FROM "DailyQuotes"
		WHERE "Date" > $1 AND "Date" <= $2
		GROUP BY "SecurityCode"
	)
),
ownership_details AS (
	SELECT security_code, share_quantity, member_id
	FROM stock_ownership_details
	WHERE is_sold = false
),
total AS (
	SELECT '{0}' AS "date", SUM(od.share_quantity * dq."ClosingPrice") AS "sum"
	FROM ownership_details od
	INNER JOIN daily_quotes dq ON od.security_code = dq."SecurityCode"
),
eddie AS (
	SELECT '{0}' AS "date", SUM(od.share_quantity * dq."ClosingPrice") AS "sum"
	FROM ownership_details od
	INNER JOIN daily_quotes dq ON od.security_code = dq."SecurityCode"
	WHERE od.member_id = 1
)
INSERT INTO daily_money_history (date, sum, eddie, unice)
SELECT
	TO_DATE(total."date",'YYYY-MM-DD') AS "date",
	"total"."sum" AS sum,
	"eddie"."sum" AS eddie,
	"total"."sum" - "eddie"."sum" AS unice
FROM total
INNER JOIN eddie ON total."date" = eddie."date"
ON CONFLICT (date) DO UPDATE SET
	sum = EXCLUDED.sum,
	eddie = EXCLUDED.eddie,
	unice = EXCLUDED.unice,
	updated_time = now();
"#,
            date
        );

        let query = sqlx::query(&sql).bind(one_month_ago).bind(date);
        let result = match tx {
            None => query.execute(database::get_connection()).await,
            Some(t) => query.execute(&mut **t).await,
        };

        match result {
            Ok(r) => Ok(r),
            Err(why) => Err(anyhow!(
                "Failed to daily_money_history::upsert({}) from database because {:?}",
                date,
                why
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::internal::logging;

    use super::*;

    #[tokio::test]
    async fn test_insert() {
        dotenv::dotenv().ok();
        logging::debug_file_async("開始 DailyMoneyHistory::upsert".to_string());
        let current_date = NaiveDate::parse_from_str("2023-08-04", "%Y-%m-%d").unwrap();
        let mut tx = database::get_tx().await.ok();
        match DailyMoneyHistory::upsert(current_date, &mut tx).await {
            Ok(r) => {
                logging::debug_file_async(format!("DailyMoneyHistory::upsert:{:#?}", r));
                tx.unwrap()
                    .commit()
                    .await
                    .expect("tx.unwrap().commit() is failed");
            }
            Err(why) => {
                logging::debug_file_async(format!(
                    "Failed to DailyMoneyHistory::upsert because {:?}",
                    why
                ));
                tx.unwrap()
                    .rollback()
                    .await
                    .expect("tx.unwrap().rollback() is failed");
            }
        }

        logging::debug_file_async("結束 DailyMoneyHistory::upsert".to_string());
    }
}
