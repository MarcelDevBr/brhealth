// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

#[test]
fn test_python_module_metadata() {
    let version = env!("CARGO_PKG_VERSION");
    assert_eq!(version, "1.0.0");
}
