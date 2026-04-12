use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use time::macros::format_description;

pub fn now_rfc3339() -> String {
    format_rfc3339(OffsetDateTime::now_utc())
}

pub fn format_rfc3339(value: OffsetDateTime) -> String {
    value.format(&Rfc3339)
        .expect("RFC3339 formatting should not fail")
}

pub fn parse_timestamp(value: &str) -> Option<OffsetDateTime> {
    OffsetDateTime::parse(value, &Rfc3339)
        .ok()
        .or_else(|| {
            OffsetDateTime::parse(
                value,
                &format_description!(
                    "[year]-[month]-[day] [hour padding:none]:[minute]:[second].[subsecond] [offset_hour sign:mandatory]:[offset_minute]:[offset_second]"
                ),
            )
            .ok()
        })
        .or_else(|| {
            OffsetDateTime::parse(
                value,
                &format_description!(
                    "[year]-[month]-[day] [hour padding:none]:[minute]:[second] [offset_hour sign:mandatory]:[offset_minute]:[offset_second]"
                ),
            )
            .ok()
        })
}
