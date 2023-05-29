use crate::internal::database;
use anyhow::*;
use chrono::{DateTime, Datelike, Duration, FixedOffset, Local, NaiveDate};
use rust_decimal::Decimal;
use sqlx::{
    postgres::{PgQueryResult, PgRow},
    Row,
};
use std::{result::Result::Ok, str::FromStr};

#[derive(sqlx::Type, sqlx::FromRow, Debug)]
pub struct Revenue {
    pub security_code: String,
    /// 當月營收
    pub monthly: Decimal,
    /// 上月營收
    pub last_month: Decimal,
    /// 去年當月營收
    pub last_year_this_month: Decimal,
    /// 當月累計營收
    pub monthly_accumulated: Decimal,
    // 去年累計營收
    pub last_year_monthly_accumulated: Decimal,
    /// 上月比較增減(%)
    pub compared_with_last_month: Decimal,
    /// 去年同月增減(%)
    pub compared_with_last_year_same_month: Decimal,
    /// 前期比較增減(%)
    pub accumulated_compared_with_last_year: Decimal,
    ///月均價
    pub avg_price: Decimal,
    /// 當月最低價
    pub lowest_price: Decimal,
    /// 當月最高價
    pub highest_price: Decimal,
    /// 那個月份的營收
    pub date: i64,
    pub create_time: DateTime<Local>,
}

impl Revenue {
    pub fn new() -> Self {
        Revenue {
            security_code: Default::default(),
            monthly: Default::default(),
            last_month: Default::default(),
            last_year_this_month: Default::default(),
            monthly_accumulated: Default::default(),
            last_year_monthly_accumulated: Default::default(),
            compared_with_last_month: Default::default(),
            compared_with_last_year_same_month: Default::default(),
            accumulated_compared_with_last_year: Default::default(),
            avg_price: Default::default(),
            lowest_price: Default::default(),
            highest_price: Default::default(),
            date: 0,
            create_time: Local::now(),
        }
    }

    pub async fn upsert(&self) -> Result<PgQueryResult> {
        let sql = r#"
insert into "Revenue" (
    "SecurityCode","Date","Monthly", "LastMonth", "LastYearThisMonth", "MonthlyAccumulated",
    "ComparedWithLastMonth", "ComparedWithLastYearSameMonth", "LastYearMonthlyAccumulated",
    "AccumulatedComparedWithLastYear", "avg_price", "lowest_price", "highest_price"

)
values (
    $1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13
)
on conflict ("SecurityCode", "Date") do update set
    "Monthly" = excluded."Monthly",
    "LastMonth" = excluded."LastMonth",
    "LastYearThisMonth" = excluded."LastYearThisMonth",
    "MonthlyAccumulated" = excluded."MonthlyAccumulated",
    "ComparedWithLastMonth" = excluded."ComparedWithLastMonth",
    "ComparedWithLastYearSameMonth" = excluded."ComparedWithLastYearSameMonth",
    "LastYearMonthlyAccumulated" = excluded."LastYearMonthlyAccumulated",
    "AccumulatedComparedWithLastYear" = excluded."AccumulatedComparedWithLastYear",
    "avg_price" = excluded."avg_price",
    "lowest_price" = excluded."lowest_price",
    "highest_price" = excluded."highest_price";
"#;
        Ok(sqlx::query(sql)
            .bind(self.security_code.as_str())
            .bind(self.date)
            .bind(self.monthly)
            .bind(self.last_month)
            .bind(self.last_year_this_month)
            .bind(self.monthly_accumulated)
            .bind(self.compared_with_last_month)
            .bind(self.compared_with_last_year_same_month)
            .bind(self.last_year_monthly_accumulated)
            .bind(self.accumulated_compared_with_last_year)
            .bind(self.avg_price)
            .bind(self.lowest_price)
            .bind(self.highest_price)
            .execute(database::get_pool()?)
            .await?)
    }
}

impl Default for Revenue {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for Revenue {
    fn clone(&self) -> Self {
        Revenue {
            security_code: self.security_code.to_string(),
            monthly: self.monthly,
            last_month: self.last_month,
            last_year_this_month: self.last_year_this_month,
            monthly_accumulated: self.monthly_accumulated,
            last_year_monthly_accumulated: self.last_year_monthly_accumulated,
            compared_with_last_month: self.compared_with_last_month,
            compared_with_last_year_same_month: self.compared_with_last_year_same_month,
            accumulated_compared_with_last_year: self.accumulated_compared_with_last_year,
            avg_price: self.avg_price,
            lowest_price: self.lowest_price,
            highest_price: self.highest_price,
            date: self.date,
            create_time: self.create_time,
        }
    }
}

//let entity: Entity = fs.into(); // 或者 let entity = Entity::from(fs);
impl From<Vec<String>> for Revenue {
    fn from(item: Vec<String>) -> Self {
        let mut e = Revenue::new();

        e.security_code = item[0].to_string();
        /*
        0公司代號	1公司名稱	2當月營收	3上月營收	4去年當月營收	5上月比較增減(%) 6去年同月增減(%) 7當月累計營收 8去年累計營收 9前期比較增減(%)
        */
        e.monthly =
            Decimal::from_str(item[2].replace([',', ' '], "").as_str()).unwrap_or_else(|err| {
                eprintln!("Failed to parse 'monthly'({}) field: {}", item[2], err);
                Default::default()
            });
        e.last_month =
            Decimal::from_str(item[3].replace([',', ' '], "").as_str()).unwrap_or_else(|err| {
                eprintln!("Failed to parse 'last_month'({}) field: {}", item[3], err);
                Default::default()
            });
        e.last_year_this_month = Decimal::from_str(item[4].replace([',', ' '], "").as_str())
            .unwrap_or_else(|err| {
                eprintln!(
                    "Failed to parse 'last_year_this_month'({}) field: {}",
                    item[4], err
                );
                Default::default()
            });
        e.monthly_accumulated = Decimal::from_str(item[7].replace([',', ' '], "").as_str())
            .unwrap_or_else(|err| {
                eprintln!(
                    "Failed to parse 'monthly_accumulated'({}) field: {}",
                    item[7], err
                );
                Default::default()
            });
        e.last_year_monthly_accumulated =
            Decimal::from_str(item[8].replace([',', ' '], "").as_str()).unwrap_or_else(|err| {
                eprintln!(
                    "Failed to parse 'last_year_monthly_accumulated'({}) field: {}",
                    item[8], err
                );
                Default::default()
            });
        e.compared_with_last_month = Decimal::from_str(item[5].replace([',', ' '], "").as_str())
            .unwrap_or_else(|err| {
                eprintln!(
                    "Failed to parse 'compared_with_last_month'({}) field: {}",
                    item[5], err
                );
                Default::default()
            });
        e.compared_with_last_year_same_month =
            Decimal::from_str(item[6].replace([',', ' '], "").as_str()).unwrap_or_else(|err| {
                eprintln!(
                    "Failed to parse 'compared_with_last_year_same_month'({}) field: {}",
                    item[6], err
                );
                Default::default()
            });
        e.accumulated_compared_with_last_year =
            Decimal::from_str(item[9].replace([',', ' '], "").as_str()).unwrap_or_else(|err| {
                eprintln!(
                    "Failed to parse 'accumulated_compared_with_last_year'({}) field: {}",
                    item[9], err
                );
                Default::default()
            });

        e
    }
}

pub async fn fetch_last_two_month() -> Result<Vec<Revenue>> {
    let now = Local::now();
    //now.offset()
    //let timezone = FixedOffset::east_opt(8 * 60 * 60).unwrap();

    let now_first_day = NaiveDate::from_ymd_opt(now.year(), now.month(), 1)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap();

    let last_month = now_first_day - Duration::minutes(1);
    let last_month_timezone = DateTime::<FixedOffset>::from_local(last_month, *now.offset());
    let two_month_ago_first_day = NaiveDate::from_ymd_opt(last_month.year(), last_month.month(), 1)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap();
    let two_month_ago = two_month_ago_first_day - Duration::minutes(1);
    let two_month_ago_timezone = DateTime::<FixedOffset>::from_local(two_month_ago, *now.offset());
    let last_month_int = (last_month_timezone.year() * 100) + last_month_timezone.month() as i32;
    let two_month_ago_int =
        (two_month_ago_timezone.year() * 100) + two_month_ago_timezone.month() as i32;
    let revenue = sqlx::query(
        r#"
select
    "SecurityCode",
    "Date",
    "Monthly",
    "LastMonth",
    "LastYearThisMonth",
    "MonthlyAccumulated",
    "LastYearMonthlyAccumulated",
    "ComparedWithLastMonth",
    "ComparedWithLastYearSameMonth",
    "AccumulatedComparedWithLastYear",
    "CreateTime",
    avg_price,
    lowest_price,
    highest_price
from "Revenue"
where
    "Date" = $1 or "Date" = $2
order by "Serial" desc
        "#,
    )
    .bind(last_month_int)
    .bind(two_month_ago_int)
    .try_map(|row: PgRow| {
        let date = row.try_get("Date")?;
        let security_code = row.try_get("SecurityCode")?;
        let monthly = row.try_get("Monthly")?;
        let last_month = row.try_get("LastMonth")?;
        let last_year_this_month = row.try_get("LastYearThisMonth")?;
        let monthly_accumulated = row.try_get("MonthlyAccumulated")?;
        let last_year_monthly_accumulated = row.try_get("LastYearMonthlyAccumulated")?;
        let compared_with_last_month = row.try_get("ComparedWithLastMonth")?;
        let compared_with_last_year_same_month = row.try_get("ComparedWithLastYearSameMonth")?;
        let accumulated_compared_with_last_year = row.try_get("AccumulatedComparedWithLastYear")?;
        let avg_price = row.try_get("avg_price")?;
        let lowest_price = row.try_get("lowest_price")?;
        let highest_price = row.try_get("highest_price")?;
        let create_time = row.try_get("CreateTime")?;
        Ok(Revenue {
            date,
            security_code,
            monthly,
            last_month,
            last_year_this_month,
            monthly_accumulated,
            last_year_monthly_accumulated,
            compared_with_last_month,
            compared_with_last_year_same_month,
            accumulated_compared_with_last_year,
            avg_price,
            lowest_price,
            highest_price,
            create_time,
        })
    })
    .fetch_all(database::get_pool()?)
    .await?;

    Ok(revenue)
}

pub async fn rebuild_revenue_last_date() -> Result<PgQueryResult> {
    let sql = r#"
--SET TIMEZONE = 'Asia/Taipei';

WITH r AS (
    SELECT
        "SecurityCode",
        MAX("Date") AS date
    FROM
        "Revenue"
    GROUP BY
        "SecurityCode"
)
INSERT INTO revenue_last_date
SELECT
    "Revenue"."SecurityCode",
    "Revenue"."Serial"
FROM
    "Revenue"
    INNER JOIN r ON r."SecurityCode" = "Revenue"."SecurityCode"
    AND r.date = "Revenue"."Date"
ON CONFLICT (security_code)
DO UPDATE SET
    serial = excluded.serial,
    created_time = now();
"#;
    Ok(sqlx::query(sql).execute(database::get_pool()?).await?)
}

#[cfg(test)]
mod tests {
    //use crate::internal::database::table::revenue;

    use chrono::{DateTime, Datelike, Duration, FixedOffset, Local, NaiveDate};
    use rust_decimal::Decimal;
    use std::str::FromStr;
    //use chrono::{Datelike, Local, NaiveDate};
    use crate::internal::database::table::revenue::{
        fetch_last_two_month, rebuild_revenue_last_date,
    };
    use crate::internal::logging;

    #[tokio::test]
    async fn test_date() {
        // 取得本月的第一天
        let now = Local::now();
        let first_day_of_month = NaiveDate::from_ymd_opt(now.year(), now.month(), 1);

        // 取得上個月的第一天
        let last_month = if now.month() == 1 {
            NaiveDate::from_ymd_opt(now.year() - 1, 12, 1)
        } else {
            NaiveDate::from_ymd_opt(now.year(), now.month() - 1, 1)
        };

        // 取得二個月前的第一天
        let two_month_ago = if now.month() <= 2 {
            NaiveDate::from_ymd_opt(now.year() - 1, now.month() + 10, 1)
        } else {
            NaiveDate::from_ymd_opt(now.year(), now.month() - 2, 1)
        };

        println!("This month's first day: {:?}", first_day_of_month);
        println!("Last month's first day: {:?}", last_month);
        println!("Two months ago first day: {:?}", two_month_ago);

        let now_first_day = NaiveDate::from_ymd_opt(now.year(), now.month(), 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();

        let last_month = now_first_day - Duration::minutes(1);
        let last_month_timezone = DateTime::<FixedOffset>::from_local(last_month, *now.offset());
        let two_month_ago_first_day =
            NaiveDate::from_ymd_opt(last_month.year(), last_month.month(), 1)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap();
        let two_month_ago = two_month_ago_first_day - Duration::minutes(1);
        let two_month_ago_timezone =
            DateTime::<FixedOffset>::from_local(two_month_ago, *now.offset());
        println!("This month's first day: {:?}", now_first_day);
        println!("Last month's first day: {:?}", last_month_timezone);
        println!("Two months ago first day: {:?}", two_month_ago_timezone);
    }

    #[tokio::test]
    async fn test_fetch() {
        dotenv::dotenv().ok();
        logging::info_file_async("開始 fetch_last_two_month".to_string());

        let m = Decimal::from_str("0.00".replace([',', ' '], "").as_str()).unwrap_or_else(|err| {
            eprintln!("Failed to parse 'compared_with_last_month' field: {}", err);
            Default::default()
        });
        println!("m={}", m);
        match fetch_last_two_month().await {
            Ok(result) => {
                for e in result {
                    logging::info_file_async(format!("{:?} ", e));
                }
            }
            Err(why) => {
                logging::error_file_async(format!("because {:?}", why));
            }
        }
        /* if let Ok(result) = r {
            for e in result {
                logging::info_file_async(format!("{:#?} ", e));
            }
        }*/
    }

    #[tokio::test]
    async fn test_rebuild_revenue_last_date() {
        dotenv::dotenv().ok();
        logging::info_file_async("開始 test_rebuild_revenue_last_date".to_string());
        match rebuild_revenue_last_date().await {
            Ok(result) => {
                logging::info_file_async(format!(
                    "rebuild_revenue_last_date:{:?} ",
                    result.rows_affected()
                ));
            }
            Err(why) => {
                logging::error_file_async(format!(
                    "Failed to rebuild_revenue_last_date because {:?}",
                    why
                ));
            }
        }
    }
}