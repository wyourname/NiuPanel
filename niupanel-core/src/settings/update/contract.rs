use super::*;

pub(super) fn parse_plain_version(value: &str, label: &str) -> Result<Version> {
    let version = Version::parse(value)
        .map_err(|error| AppError::ValidationError(format!("{label} 不是有效版本号: {error}")))?;
    if !version.pre.is_empty() || !version.build.is_empty() {
        return Err(AppError::ValidationError(format!(
            "{label} 必须使用纯数字版本号，不能包含预发布或构建后缀"
        )));
    }
    Ok(version)
}

fn parse_panel_version(value: &str, channel: &str) -> Result<Version> {
    let version = Version::parse(value).map_err(|error| {
        AppError::ValidationError(format!("Panel 版本不是有效 SemVer: {error}"))
    })?;
    if !version.build.is_empty() {
        return Err(AppError::ValidationError(
            "Panel 版本不能包含构建后缀".to_string(),
        ));
    }
    if channel == "stable" && !version.pre.is_empty() {
        return Err(AppError::ValidationError(
            "stable 通道不能指向预发布版本".to_string(),
        ));
    }
    Ok(version)
}

fn validate_asset_descriptor(name: &str, url: &str, sha256: &str, size: u64) -> Result<()> {
    if name.is_empty()
        || Path::new(name).file_name().and_then(|value| value.to_str()) != Some(name)
        || !url.starts_with("https://")
        || size == 0
        || sha256.len() != 64
        || !sha256.chars().all(|value| value.is_ascii_hexdigit())
    {
        return Err(AppError::ValidationError(format!(
            "更新索引中的资产 {name} 描述不合法"
        )));
    }
    Ok(())
}

fn release_url_matches(url: &str, tag: &str) -> bool {
    reqwest::Url::parse(url).is_ok_and(|url| {
        url.scheme() == "https"
            && url.query().is_none()
            && url.fragment().is_none()
            && url.path().ends_with(&format!("/releases/tag/{tag}"))
    })
}

fn expected_download_url(release_url: &str, asset_name: &str) -> Option<String> {
    let (repository_url, tag) = release_url.split_once("/releases/tag/")?;
    Some(format!(
        "{repository_url}/releases/download/{tag}/{asset_name}"
    ))
}

pub(super) fn validate_update_channel_index(
    index: &UpdateChannelIndex,
    expected_channel: &str,
) -> Result<()> {
    if index.schema_version != UPDATE_CHANNEL_INDEX_SCHEMA_VERSION {
        return Err(AppError::ValidationError(format!(
            "不支持的更新索引 schema {}",
            index.schema_version
        )));
    }
    if index.channel != expected_channel || !matches!(index.channel.as_str(), "preview" | "stable")
    {
        return Err(AppError::ValidationError(format!(
            "更新索引通道 {} 与请求通道 {} 不一致",
            index.channel, expected_channel
        )));
    }
    chrono::DateTime::parse_from_rfc3339(&index.updated_at)
        .map_err(|error| AppError::ValidationError(format!("更新索引时间戳不合法: {error}")))?;
    let panel_version = parse_panel_version(&index.release.version, expected_channel)?;
    if index.release.tag != format!("v{}", index.release.version) {
        return Err(AppError::ValidationError(
            "Panel Tag 与版本不一致".to_string(),
        ));
    }
    if !release_url_matches(&index.release.release_url, &index.release.tag) {
        return Err(AppError::ValidationError(
            "Panel Release 地址不合法".to_string(),
        ));
    }
    if index.release.launcher_protocol != niupanel_common::version::RELEASE_PROTOCOL_VERSION {
        return Err(AppError::ValidationError(format!(
            "Panel launcher 协议 {} 与当前协议不兼容",
            index.release.launcher_protocol
        )));
    }
    let minimum_supported = parse_plain_version(MINIMUM_UPDATE_CORE_VERSION, "最低支持 Core 版本")?;
    let core_version = parse_plain_version(&index.release.core.version, "Core 版本")?;
    if core_version < minimum_supported {
        return Err(AppError::ValidationError(format!(
            "Core {} 低于最低支持更新基线 {}",
            index.release.core.version, MINIMUM_UPDATE_CORE_VERSION
        )));
    }
    if index.release.core.tag != format!("Core-v{}", index.release.core.version) {
        return Err(AppError::ValidationError(
            "Core Tag 与版本不一致".to_string(),
        ));
    }
    if index.release.web.tag != format!("web-v{}", index.release.web.version) {
        return Err(AppError::ValidationError(
            "Web Tag 与版本不一致".to_string(),
        ));
    }
    if !release_url_matches(&index.release.core.release_url, &index.release.core.tag)
        || !release_url_matches(&index.release.web.release_url, &index.release.web.tag)
    {
        return Err(AppError::ValidationError(
            "更新索引中的 Release 地址不合法".to_string(),
        ));
    }
    if index.release.core.api_contract != API_CONTRACT_VERSION
        || index.release.web.api_contract != API_CONTRACT_VERSION
    {
        return Err(AppError::ValidationError(format!(
            "更新索引 API contract 不兼容：Core={}, Web={}, 当前={}",
            index.release.core.api_contract, index.release.web.api_contract, API_CONTRACT_VERSION
        )));
    }
    if index.release.core.api_contract != index.release.web.api_contract {
        return Err(AppError::ValidationError(
            "更新索引 Core 与 Web 的 API contract 不一致".to_string(),
        ));
    }
    if index.release.core.launcher_protocol != niupanel_common::version::RELEASE_PROTOCOL_VERSION {
        return Err(AppError::ValidationError(format!(
            "更新索引 Core launcher 协议 {} 与当前协议不兼容",
            index.release.core.launcher_protocol
        )));
    }

    for (architecture, target) in [
        ("x86_64", "x86_64-unknown-linux-musl"),
        ("aarch64", "aarch64-unknown-linux-musl"),
        ("armv7", "armv7-unknown-linux-musleabihf"),
    ] {
        let asset = index.release.core.assets.get(architecture).ok_or_else(|| {
            AppError::ValidationError(format!("更新索引缺少 {architecture} Core 资产"))
        })?;
        validate_asset_descriptor(&asset.name, &asset.url, &asset.sha256, asset.size)?;
        if asset.name != format!("niupanel_linux_{architecture}.tar.gz") {
            return Err(AppError::ValidationError(format!(
                "Core {architecture} 资产文件名不符合发布契约"
            )));
        }
        if asset.target != target {
            return Err(AppError::ValidationError(format!(
                "Core 资产 {} 的 target {} 与架构 {} 不一致",
                asset.name, asset.target, architecture
            )));
        }
        if expected_download_url(&index.release.core.release_url, &asset.name).as_deref()
            != Some(asset.url.as_str())
        {
            return Err(AppError::ValidationError(format!(
                "Core 资产 {} 不属于声明的不可变 Release",
                asset.name
            )));
        }
    }
    validate_asset_descriptor(
        &index.release.web.asset.name,
        &index.release.web.asset.url,
        &index.release.web.asset.sha256,
        index.release.web.asset.size,
    )?;
    if index.release.web.asset.name != format!("niupanel_web_{}.tar.gz", index.release.web.version)
    {
        return Err(AppError::ValidationError(
            "Web 资产文件名不符合发布契约".to_string(),
        ));
    }
    if expected_download_url(
        &index.release.web.release_url,
        &index.release.web.asset.name,
    )
    .as_deref()
        != Some(index.release.web.asset.url.as_str())
    {
        return Err(AppError::ValidationError(
            "Web 资产不属于声明的不可变 Release".to_string(),
        ));
    }
    let minimum = parse_plain_version(&index.release.web.core.min, "Web 最低 Core 版本")?;
    if core_version < minimum {
        return Err(AppError::ValidationError(format!(
            "Core {} 低于 Web 最低要求 {}",
            index.release.core.version, index.release.web.core.min
        )));
    }
    if let Some(maximum) = index.release.web.core.max.as_deref() {
        let maximum = parse_plain_version(maximum, "Web 最高 Core 版本")?;
        if core_version > maximum {
            return Err(AppError::ValidationError(format!(
                "Core {} 高于 Web 最高支持版本 {}",
                index.release.core.version, maximum
            )));
        }
    }
    if panel_version < Version::new(0, 8, 0) {
        return Err(AppError::ValidationError(
            "Panel 更新仅支持 0.8.0 及以上版本".to_string(),
        ));
    }
    Ok(())
}

pub(super) fn core_asset_for_current_target(
    index: &UpdateChannelIndex,
) -> Result<&UpdateCoreAsset> {
    let architecture = match std::env::consts::ARCH {
        "arm" => "armv7",
        value => value,
    };
    index.release.core.assets.get(architecture).ok_or_else(|| {
        AppError::ExternalApi(format!(
            "Core {} 不支持当前架构 {}",
            index.release.core.version, architecture
        ))
    })
}
