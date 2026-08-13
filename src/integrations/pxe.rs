//! PXE / iPXE config generation.

use crate::config::PxeSettings;
use crate::db::models::image::ImageRow;
use crate::db::models::node::NodeRow;
use crate::error::{AppError, IntegrationKind};
use async_trait::async_trait;
use std::path::Path;
use tokio::fs;
use tokio::io::AsyncWriteExt;

#[async_trait]
pub trait Pxe: Send + Sync {
    async fn render_menu(&self, images: &[ImageRow], nodes: &[NodeRow]) -> Result<String, AppError>;
    async fn render_per_host(
        &self,
        node: &NodeRow,
        image: Option<&ImageRow>,
    ) -> Result<String, AppError>;
    async fn write_menu(&self, content: &str) -> Result<(), AppError>;
    async fn write_per_host(&self, mac: &str, content: &str) -> Result<(), AppError>;
    async fn render_grub(&self, image: &ImageRow) -> Result<String, AppError>;
}

pub struct PxeIntegration {
    settings: PxeSettings,
    mock: bool,
}

impl PxeIntegration {
    pub fn new(settings: PxeSettings, mock: bool) -> Self {
        Self { settings, mock }
    }

    pub fn settings(&self) -> &PxeSettings {
        &self.settings
    }

    pub fn mac_to_ipxe_filename(mac: &str) -> String {
        let cleaned: String = mac
            .chars()
            .filter(|c| c.is_ascii_hexdigit())
            .map(|c| c.to_ascii_lowercase())
            .collect();
        format!("01-{cleaned}")
    }
}

#[async_trait]
impl Pxe for PxeIntegration {
    async fn render_menu(&self, images: &[ImageRow], nodes: &[NodeRow]) -> Result<String, AppError> {
        let mut out = String::new();
        out.push_str("#!ipxe\n");
        out.push_str(&format!(
            "set menu-timeout {}\n",
            self.settings.menu_timeout_secs
        ));
        out.push_str("set garos-server http://${next-server}/garos\n");
        out.push_str(":start\n");
        out.push_str("menu Garos PXE Boot\n");
        out.push_str(&format!(
            "item --gap -- Choose an image (timeout {})\n",
            self.settings.menu_timeout_secs
        ));
        for img in images {
            let label = img.name.replace(' ', "_");
            out.push_str(&format!("item {label} {name}\n", label = label, name = img.name));
        }
        for node in nodes {
            if let Some(host) = &node.hostname {
                let label = format!("host_{}", sanitize(host));
                out.push_str(&format!("item {label} Maintenance for {host}\n"));
            }
        }
        out.push_str("choose target && goto ${target}\n");
        for img in images {
            let label = img.name.replace(' ', "_");
            let kernel = img.kernel.clone().unwrap_or_else(|| "bzImage".into());
            let args = img.kernel_args.clone().unwrap_or_default();
            out.push_str(&format!(
                ":{label}\nkernel {kernel} {args} initrd=initrd kodi=true || goto failed\nboot || goto failed\n"
            ));
        }
        out.push_str(":failed\necho Boot failed. Sleeping forever...\nsleep 86400\n");
        Ok(out)
    }

    async fn render_per_host(
        &self,
        node: &NodeRow,
        image: Option<&ImageRow>,
    ) -> Result<String, AppError> {
        let mut out = String::new();
        out.push_str("#!ipxe\n");
        if let Some(img) = image {
            let kernel = img.kernel.clone().unwrap_or_else(|| "bzImage".into());
            let args = img.kernel_args.clone().unwrap_or_default();
            out.push_str(&format!(
                "kernel http://${{next-server}}/garos/images/{}/{} {}\n",
                img.id, kernel, args
            ));
            out.push_str(&format!(
                "initrd http://${{next-server}}/garos/images/{}/initrd\n",
                img.id
            ));
        } else {
            out.push_str("echo No image assigned to this station. Sleeping...\n");
            out.push_str("sleep infinity\n");
        }
        let _ = node;
        Ok(out)
    }

    async fn write_menu(&self, content: &str) -> Result<(), AppError> {
        if self.mock {
            tracing::info!(target: "pxe", bytes = content.len(), "[mock] write menu");
            return Ok(());
        }
        write_atomic(&self.settings.tftp_root.join("menu.ipxe"), content).await
    }

    async fn write_per_host(&self, mac: &str, content: &str) -> Result<(), AppError> {
        if self.mock {
            tracing::info!(target: "pxe", %mac, bytes = content.len(), "[mock] write per-host");
            return Ok(());
        }
        let name = Self::mac_to_ipxe_filename(mac);
        write_atomic(&self.settings.tftp_root.join(format!("{name}.ipxe")), content).await
    }

    async fn render_grub(&self, image: &ImageRow) -> Result<String, AppError> {
        let kernel = image.kernel.clone().unwrap_or_else(|| "bzImage".into());
        let args = image.kernel_args.clone().unwrap_or_default();
        Ok(format!(
            "set default=0\nset timeout=5\nmenuentry 'Garos - {name}' {{\n  linux (http,${{next-server}})/garos/images/{id}/{kernel} {args}\n  initrd (http,${{next-server}})/garos/images/{id}/initrd\n}}\n",
            name = image.name,
            id = image.id,
            kernel = kernel,
            args = args,
        ))
    }
}

async fn write_atomic(path: &Path, content: &str) -> Result<(), AppError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).await.map_err(|e| AppError::Integration {
            kind: IntegrationKind::Pxe,
            message: format!("mkdir: {e}"),
        })?;
    }
    let tmp = path.with_extension("tmp");
    let mut f = fs::File::create(&tmp).await.map_err(|e| AppError::Integration {
        kind: IntegrationKind::Pxe,
        message: format!("create tmp: {e}"),
    })?;
    f.write_all(content.as_bytes())
        .await
        .map_err(|e| AppError::Integration {
            kind: IntegrationKind::Pxe,
            message: format!("write: {e}"),
        })?;
    f.sync_all()
        .await
        .map_err(|e| AppError::Integration {
            kind: IntegrationKind::Pxe,
            message: format!("sync: {e}"),
        })?;
    drop(f);
    fs::rename(&tmp, path).await.map_err(|e| AppError::Integration {
        kind: IntegrationKind::Pxe,
        message: format!("rename: {e}"),
    })?;
    Ok(())
}

fn sanitize(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use uuid::Uuid;

    fn img(name: &str) -> ImageRow {
        ImageRow {
            id: Uuid::now_v7().to_string(),
            name: name.into(),
            description: None,
            nixos_version: Some("24.05".into()),
            kernel: Some("bzImage".into()),
            kernel_args: Some("quiet splash".into()),
            size_mb: Some(512),
            status: "ready".into(),
            packages_json: None,
            custom_nix: None,
            author_id: None,
            version: "1.0.0".into(),
            parent_id: None,
            build_log: None,
            published_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn renders_menu() {
        let p = PxeIntegration::new(PxeSettings::default(), true);
        let menu = p.render_menu(&[img("base"), img("dev")], &[]).await.unwrap();
        assert!(menu.contains("item base base"));
        assert!(menu.contains("item dev dev"));
        assert!(menu.contains("set menu-timeout 10"));
    }

    #[test]
    fn mac_filename() {
        assert_eq!(
            PxeIntegration::mac_to_ipxe_filename("AA:BB:CC:DD:EE:FF"),
            "01-aabbccddeeff"
        );
    }
}
