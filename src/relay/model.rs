use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Deserialize, Serialize, Validate)]
pub struct MetricInput {

    pub name: String,

    pub labels: HashMap<String, String>,

    pub metric: f32

}

#[derive(Deserialize, Serialize, Validate)]
pub struct GroupResultInput {

    #[validate(length(max = 250))]
    pub name: String,

    pub working: bool,

    pub has_warnings: bool,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub error_message: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub error_detail: Option<String>,

    pub metrics: Vec<MetricInput>

}

#[derive(PartialEq, Debug)]
pub enum ResultCategory {
    Success,
    Warning,
    Fail
}

#[derive(PartialEq, Debug)]
pub struct TestResult {

    pub target: String,

    pub result: ResultCategory,

    pub metrics: Option<HashMap<String, f32>>

}

impl TestResult {
    
    pub fn success<M>(target_name: M) -> TestResult where M: Into<String> {

        TestResult {
            target: target_name.into(),
            result: ResultCategory::Success,
            metrics: None
        }
    }

    pub fn warning<M>(target_name: M) -> TestResult where M: Into<String> {

        TestResult {
            target: target_name.into(),
            result: ResultCategory::Warning,
            metrics: None
        }
    }

    pub fn fail<M>(target_name: M) -> TestResult where M: Into<String> {

        TestResult {
            target: target_name.into(),
            result: ResultCategory::Fail,
            metrics: None
        }
    }

    pub fn build<M>(target_name: M, result: ResultCategory, metrics: Option<HashMap<String, f32>>) -> TestResult where M: Into<String> {

        TestResult {
            target: target_name.into(),
            result,
            metrics
        }
    }

}
