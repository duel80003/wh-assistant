use anyhow::{anyhow, Result};
use serde::Deserialize;
use std::env;
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};

const GITHUB_OWNER: &str = "duel80003";
const GITHUB_REPO: &str = "wh-assistant";

#[derive(Debug, Clone, PartialEq, Default)]
pub struct UpdateInfo {
    pub has_update: bool,
    pub current_version: String,
    pub latest_version: String,
    pub release_notes: String,
    pub download_url: Option<String>,
    pub html_url: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum UpdateStatus {
    Idle,
    Downloading { progress: u8 },
    Extracting,
    ReadyToRestart,
    Failed(String),
}

#[derive(Deserialize, Debug)]
struct GitHubRelease {
    tag_name: String,
    body: Option<String>,
    html_url: String,
    assets: Vec<GitHubAsset>,
}

#[derive(Deserialize, Debug)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
}

pub struct UpdaterService;

impl UpdaterService {
    /// 檢查 GitHub Releases 是否有新版本
    pub async fn check_for_updates() -> Result<Option<UpdateInfo>> {
        let current_version = env!("CARGO_PKG_VERSION"); // e.g. "0.1.0"
        let url = format!(
            "https://api.github.com/repos/{}/{}/releases/latest",
            GITHUB_OWNER, GITHUB_REPO
        );

        let client = reqwest::Client::builder()
            .user_agent(format!("WHassistant-Updater/{}", current_version))
            .build()?;

        let resp = client.get(&url).send().await?;
        if !resp.status().is_success() {
            return Ok(None);
        }

        let release = resp.json::<GitHubRelease>().await?;
        let clean_remote = release.tag_name.trim().trim_start_matches('v');

        if is_newer_version(clean_remote, current_version) {
            // 優先尋找符合 Windows 或當前架構的 zip 安裝包
            let download_url = release
                .assets
                .iter()
                .find(|a| {
                    let name = a.name.to_lowercase();
                    #[cfg(target_os = "windows")]
                    {
                        name.ends_with(".zip") && (name.contains("windows") || name.contains("win"))
                    }
                    #[cfg(not(target_os = "windows"))]
                    {
                        name.ends_with(".zip")
                    }
                })
                .or_else(|| release.assets.first())
                .map(|a| a.browser_download_url.clone());

            Ok(Some(UpdateInfo {
                has_update: true,
                current_version: current_version.to_string(),
                latest_version: release.tag_name.clone(),
                release_notes: release.body.unwrap_or_default(),
                download_url,
                html_url: release.html_url,
            }))
        } else {
            Ok(None)
        }
    }

    /// 下載最新發行包並進行本機熱替換
    pub async fn download_and_install_update(download_url: &str) -> Result<()> {
        let current_exe = env::current_exe()?;
        let app_dir = current_exe
            .parent()
            .ok_or_else(|| anyhow!("無法取得目前執行檔目錄"))?;

        // 1. 下載 zip 檔案到記憶體
        let client = reqwest::Client::builder()
            .user_agent("WHassistant-Updater")
            .build()?;
        let bytes = client.get(download_url).send().await?.bytes().await?;

        // 2. 解壓縮
        let reader = Cursor::new(bytes);
        let mut archive = zip::ZipArchive::new(reader)?;

        // 建立臨時解壓目錄
        let temp_extract_dir = app_dir.join(".update_temp");
        if temp_extract_dir.exists() {
            let _ = fs::remove_dir_all(&temp_extract_dir);
        }
        fs::create_dir_all(&temp_extract_dir)?;

        archive.extract(&temp_extract_dir)?;

        // 3. 在解壓後的目錄中找到新的執行檔
        let new_exe = find_executable_in_dir(&temp_extract_dir)?
            .ok_or_else(|| anyhow!("更新壓縮包內未包含執行檔"))?;

        // 4. 熱替換執行檔（Self-Update replacement）
        // 在 Windows 上，當前運行的 exe 無法直接 overwrite，但允許被 rename！
        // 因此先將舊 exe 重新命名為 .old，再把新 exe 移過來。
        let old_backup = current_exe.with_extension("old");
        if old_backup.exists() {
            let _ = fs::remove_file(&old_backup);
        }

        fs::rename(&current_exe, &old_backup)?;
        fs::copy(&new_exe, &current_exe)?;

        // 5. 若解壓目錄包含 assets/，亦同步複製更新
        let new_assets = find_dir_by_name(&temp_extract_dir, "assets")?;
        if let Some(assets_src) = new_assets {
            let assets_dst = app_dir.join("assets");
            let _ = copy_dir_all(&assets_src, &assets_dst);
        }

        // 清理暫存檔
        let _ = fs::remove_dir_all(&temp_extract_dir);

        Ok(())
    }

    /// 重啟應用程式
    pub fn restart_app() -> Result<()> {
        let current_exe = env::current_exe()?;
        std::process::Command::new(current_exe).spawn()?;
        std::process::exit(0);
    }
}

/// 比較兩個語意化版本號（例如 "0.2.0" 是否大於 "0.1.0"）
fn is_newer_version(remote: &str, current: &str) -> bool {
    let parse =
        |v: &str| -> Vec<u64> { v.split(['.', '-']).filter_map(|s| s.parse().ok()).collect() };
    parse(remote) > parse(current)
}

fn find_executable_in_dir(dir: &Path) -> Result<Option<PathBuf>> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            let name = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_lowercase();
            #[cfg(target_os = "windows")]
            {
                if name.ends_with(".exe") {
                    return Ok(Some(path));
                }
            }
            #[cfg(not(target_os = "windows"))]
            {
                if name.contains("whassistant") || name.contains("w-hassistant") {
                    return Ok(Some(path));
                }
            }
        } else if path.is_dir() {
            if let Ok(Some(found)) = find_executable_in_dir(&path) {
                return Ok(Some(found));
            }
        }
    }
    Ok(None)
}

fn find_dir_by_name(dir: &Path, target_name: &str) -> Result<Option<PathBuf>> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            if path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .eq_ignore_ascii_case(target_name)
            {
                return Ok(Some(path));
            }
            if let Ok(Some(found)) = find_dir_by_name(&path, target_name) {
                return Ok(Some(found));
            }
        }
    }
    Ok(None)
}

fn copy_dir_all(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_all(&from, &to)?;
        } else {
            let _ = fs::copy(&from, &to);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_comparison() {
        assert!(is_newer_version("0.2.0", "0.1.0"));
        assert!(is_newer_version("1.0.0", "0.9.9"));
        assert!(is_newer_version("0.1.1", "0.1.0"));
        assert!(!is_newer_version("0.1.0", "0.1.0"));
        assert!(!is_newer_version("0.0.9", "0.1.0"));
    }
}
