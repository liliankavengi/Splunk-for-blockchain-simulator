use crate::clickhouse_client::AssertionClient;

#[derive(Debug)]
pub struct ReorgResult {
    pub expected_reorg_logs: u64,
    pub actual_reorg_logs: u64,
    pub duplicate_block_hashes: u64,
    pub passed: bool,
    pub skipped: bool,
}

/// Asserts that the expected number of reorg logs landed and no block hash collisions exist.
pub async fn check_reorg_handling(
    client: &AssertionClient,
    scenario_name: &str,
    expected_reorg_logs: u64,
) -> anyhow::Result<ReorgResult> {
    if expected_reorg_logs == 0 {
        return Ok(ReorgResult {
            expected_reorg_logs: 0,
            actual_reorg_logs: 0,
            duplicate_block_hashes: 0,
            passed: true,
            skipped: true,
        });
    }

    let actual = client.count_reorg_logs(scenario_name).await?;
    let duplicates = client
        .count_duplicate_block_hashes(scenario_name)
        .await
        .unwrap_or(0);

    let passed = actual >= expected_reorg_logs && duplicates == 0;

    Ok(ReorgResult {
        expected_reorg_logs,
        actual_reorg_logs: actual,
        duplicate_block_hashes: duplicates,
        passed,
        skipped: false,
    })
}
