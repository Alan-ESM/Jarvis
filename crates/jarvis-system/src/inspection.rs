use crate::processes;
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PcInspectionReport {
    pub processes_csv: String,
    pub services_text: String,
    pub notes: Vec<String>,
}

#[derive(Debug, Default, Clone)]
pub struct PcInspector;

impl PcInspector {
    pub fn inspect_overview(&self) -> Result<PcInspectionReport> {
        Ok(PcInspectionReport {
            processes_csv: processes::tasklist_csv()?,
            services_text: processes::services_text()?,
            notes: vec![
                "Registry, installed applications, drivers, and deep settings require the Windows API implementation step.".to_string(),
            ],
        })
    }
}
