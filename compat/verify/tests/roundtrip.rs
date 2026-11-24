use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use sqlx::{Connection, Row, SqliteConnection};

#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq)]
struct Rec {
    at: DateTime<Utc>,
    naive: NaiveDateTime,
}

fn sample() -> Rec {
    Rec {
        at: Utc.with_ymd_and_hms(2024, 2, 29, 23, 59, 58).unwrap() + chrono::Duration::nanoseconds(123_456_000),
        naive: chrono::NaiveDate::from_ymd_opt(1999, 12, 31).unwrap().and_hms_micro_opt(1, 2, 3, 456_789).unwrap(),
    }
}

#[test]
fn serde_json_round_trip() {
    let r = sample();
    let json = serde_json::to_string(&r).unwrap();
    assert!(json.contains("2024-02-29T23:59:58.123456Z"), "{json}");
    assert_eq!(serde_json::from_str::<Rec>(&json).unwrap(), r);
}

#[tokio::test(flavor = "current_thread")]
async fn sqlx_sqlite_round_trip() {
    let mut c = SqliteConnection::connect("sqlite::memory:").await.unwrap();
    sqlx::query("CREATE TABLE t (at TIMESTAMP NOT NULL, naive DATETIME NOT NULL)")
        .execute(&mut c)
        .await
        .unwrap();
    let r = sample();
    sqlx::query("INSERT INTO t (at, naive) VALUES (?, ?)")
        .bind(r.at)
        .bind(r.naive)
        .execute(&mut c)
        .await
        .unwrap();
    let row = sqlx::query("SELECT at, naive FROM t").fetch_one(&mut c).await.unwrap();
    let at: DateTime<Utc> = row.get("at");
    let naive: NaiveDateTime = row.get("naive");
    assert_eq!(at, r.at);
    assert_eq!(naive, r.naive);
}

// The shim's DateTime<Utc> is the very type ironchrono exports.
#[test]
fn same_type_as_ironchrono() {
    fn takes(_: iron::DateTime<iron::Utc>) {}
    takes(sample().at);
}
