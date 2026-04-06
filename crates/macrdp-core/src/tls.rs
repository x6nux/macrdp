//! TLS certificate generation and management

use anyhow::{Context, Result};
use rcgen::{CertificateParams, DistinguishedName, KeyPair};
use std::fs;
use std::path::{Path, PathBuf};

use crate::config::config_dir;

/// Generate a self-signed certificate and write PEM files
pub fn generate_self_signed_cert(cert_path: &Path, key_path: &Path) -> Result<()> {
    let mut params = CertificateParams::default();
    let mut dn = DistinguishedName::new();
    dn.push(rcgen::DnType::CommonName, "macrdp");
    dn.push(rcgen::DnType::OrganizationName, "macrdp");
    params.distinguished_name = dn;

    params.not_after = rcgen::date_time_ymd(2027, 3, 24);

    params.subject_alt_names = vec![
        rcgen::SanType::DnsName("localhost".try_into()?),
        rcgen::SanType::IpAddress(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST)),
    ];

    let key_pair = KeyPair::generate()?;
    let cert = params.self_signed(&key_pair)?;

    if let Some(parent) = cert_path.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(cert_path, cert.pem())?;
    fs::write(key_path, key_pair.serialize_pem())?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(key_path, fs::Permissions::from_mode(0o600))?;
    }

    tracing::info!(?cert_path, ?key_path, "Generated self-signed TLS certificate");
    Ok(())
}

/// Ensure TLS cert and key files exist. Generate if missing.
pub fn ensure_tls_files(
    cert_path: Option<&Path>,
    key_path: Option<&Path>,
) -> Result<(PathBuf, PathBuf)> {
    let tls_dir = config_dir().join("tls");
    let cert = cert_path
        .map(PathBuf::from)
        .unwrap_or_else(|| tls_dir.join("cert.pem"));
    let key = key_path
        .map(PathBuf::from)
        .unwrap_or_else(|| tls_dir.join("key.pem"));

    if !cert.exists() || !key.exists() {
        generate_self_signed_cert(&cert, &key)
            .context("Failed to generate self-signed certificate")?;
    }

    Ok((cert, key))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_generate_self_signed_cert() {
        let dir = TempDir::new().unwrap();
        let cert_path = dir.path().join("cert.pem");
        let key_path = dir.path().join("key.pem");

        generate_self_signed_cert(&cert_path, &key_path).unwrap();

        assert!(cert_path.exists());
        assert!(key_path.exists());

        let cert_content = fs::read_to_string(&cert_path).unwrap();
        assert!(cert_content.contains("BEGIN CERTIFICATE"));

        let key_content = fs::read_to_string(&key_path).unwrap();
        assert!(key_content.contains("BEGIN PRIVATE KEY"));
    }

    #[test]
    fn test_ensure_tls_files_creates_when_missing() {
        let dir = TempDir::new().unwrap();
        let cert_path = dir.path().join("cert.pem");
        let key_path = dir.path().join("key.pem");

        let (c, k) = ensure_tls_files(Some(&cert_path), Some(&key_path)).unwrap();
        assert_eq!(c, cert_path);
        assert_eq!(k, key_path);
        assert!(cert_path.exists());
    }
}
