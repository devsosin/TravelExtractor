use serde::Deserialize;

pub enum ReportType {
    Filter,
    Extract,
}

impl ReportType {
    pub fn to_str(&self) -> &str {
        match self {
            Self::Filter => "filter",
            Self::Extract => "extract",
            _ => "",
        }
    }
}

#[derive(Deserialize)]
pub enum PostType {
    Course,
    Spot,
    Tip,
}

impl PostType {
    pub fn to_str(&self) -> &str {
        match self {
            PostType::Course => "course",
            PostType::Spot => "spot",
            PostType::Tip => "tip",
        }
    }
}
