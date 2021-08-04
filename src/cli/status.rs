use crate::common::error::Error;
use crate::server::storage::RegionSummary;
use super::utils::api_get;

pub async fn display_status(base_url: &str, token: &str) -> Result<(), Error> {

    let region_summary: RegionSummary = api_get(base_url, token, "api/v1/analytics").await?;

    for region_item in region_summary.regions.iter() {
        println!("{}\t{}\t\t{}", region_item.name, region_item.status, region_item.last_update);

        for group in region_summary.groups.iter() {

            if !group.name.starts_with(&region_item.name) {
                continue;
            }

            println!(" - {}\t{}", group.name, group.status);   
        }
    }

    Ok(())
}
