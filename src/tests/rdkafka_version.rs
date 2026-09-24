//! 回归测试：链接进二进制的 librdkafka 必须是本地 vendored 的版本
//!
//! rdkafka-sys 走 [patch.crates-io] 指向 ../rdkafka-sys（2.15.1，独立仓库 sufar/rdkafka-sys），
//! 若 patch 失效/被覆盖回 crates.io 版本，这里会立刻暴露。

use rdkafka::util::get_rdkafka_version;

#[test]
fn librdkafka_version_matches_vendored() {
    let (_num, version_str) = get_rdkafka_version();
    println!("linked librdkafka version: {}", version_str);
    // 与 ../rdkafka-sys/Cargo.toml 的 `4.10.0+<此版本>` 保持一致
    assert!(
        version_str.starts_with("2.15."),
        "expected vendored librdkafka 2.15.x, got {}",
        version_str
    );
}
