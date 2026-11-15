use super::LocalStore;
use kusi_application::local::{LocalAsset, LocalError};
use kusi_domain::local::LocalUser;
use sha2::{Digest, Sha256};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(FromRow)]
struct AssetRow {
    id: Uuid,
    label: String,
    filename: String,
    media_type: String,
    size_bytes: i64,
    sha256: String,
    simulated: bool,
    creative_job_id: Option<Uuid>,
    created_at: i64,
}
impl From<AssetRow> for LocalAsset {
    fn from(a: AssetRow) -> Self {
        Self {
            id: a.id,
            label: a.label,
            filename: a.filename,
            media_type: a.media_type,
            size_bytes: a.size_bytes,
            sha256: a.sha256,
            simulated: a.simulated,
            creative_job_id: a.creative_job_id,
            created_at: a.created_at,
        }
    }
}
impl LocalStore {
    pub async fn assets(&self, org: Uuid) -> Result<Vec<LocalAsset>, LocalError> {
        Ok(sqlx::query_as::<_,AssetRow>("SELECT id,label,filename,media_type,size_bytes,sha256,simulated,creative_job_id,extract(epoch from created_at)::bigint as created_at FROM local_assets WHERE organization_id=$1 ORDER BY created_at DESC LIMIT 200").bind(org).fetch_all(&self.pool).await.map_err(|_|LocalError::Unavailable)?.into_iter().map(Into::into).collect())
    }
    pub async fn asset(&self, org: Uuid, id: Uuid) -> Result<LocalAsset, LocalError> {
        sqlx::query_as::<_,AssetRow>("SELECT id,label,filename,media_type,size_bytes,sha256,simulated,creative_job_id,extract(epoch from created_at)::bigint as created_at FROM local_assets WHERE organization_id=$1 AND id=$2").bind(org).bind(id).fetch_optional(&self.pool).await.map_err(|_|LocalError::Unavailable)?.map(Into::into).ok_or(LocalError::NotFound)
    }
    pub(crate) async fn asset_path(
        &self,
        org: Uuid,
        id: Uuid,
    ) -> Result<std::path::PathBuf, LocalError> {
        let key: Option<String> = sqlx::query_scalar(
            "SELECT storage_key FROM local_assets WHERE organization_id=$1 AND id=$2",
        )
        .bind(org)
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| LocalError::Unavailable)?;
        let key = key.ok_or(LocalError::NotFound)?;
        if key.contains('/') || key.contains('\\') || key.starts_with('.') {
            return Err(LocalError::Unavailable);
        }
        Ok(self.root.join("assets").join(key))
    }
    pub async fn asset_bytes(
        &self,
        org: Uuid,
        id: Uuid,
    ) -> Result<(LocalAsset, Vec<u8>), LocalError> {
        let asset = self.asset(org, id).await?;
        let path = self.asset_path(org, id).await?;
        let bytes = tokio::fs::read(path)
            .await
            .map_err(|_| LocalError::NotFound)?;
        if bytes.len() > 20 * 1024 * 1024 {
            return Err(LocalError::Unavailable);
        }
        Ok((asset, bytes))
    }
    pub async fn upload(
        &self,
        actor: &LocalUser,
        label: &str,
        filename: &str,
        bytes: Vec<u8>,
    ) -> Result<LocalAsset, LocalError> {
        if !actor.role.may_edit() {
            return Err(LocalError::Forbidden);
        }
        if label.trim().is_empty()
            || label.chars().count() > 100
            || filename.is_empty()
            || filename.len() > 120
            || filename.contains(['/', '\\'])
            || filename.chars().any(char::is_control)
            || bytes.is_empty()
            || bytes.len() > 10 * 1024 * 1024
        {
            return Err(LocalError::Invalid);
        }
        let id = Uuid::new_v4();
        let is_mp4 = bytes.get(4..8) == Some(b"ftyp");
        let is_webm = bytes.starts_with(&[0x1a, 0x45, 0xdf, 0xa3]);
        let (bytes, mime, extension) = if is_mp4 || is_webm {
            (
                bytes,
                if is_mp4 { "video/mp4" } else { "video/webm" },
                if is_mp4 { "mp4" } else { "webm" },
            )
        } else {
            let encoded = tokio::task::spawn_blocking(move || {
                let mut reader = image::ImageReader::new(std::io::Cursor::new(bytes))
                    .with_guessed_format()
                    .map_err(|_| LocalError::Invalid)?;
                if !matches!(
                    reader.format(),
                    Some(
                        image::ImageFormat::Png
                            | image::ImageFormat::Jpeg
                            | image::ImageFormat::WebP
                    )
                ) {
                    return Err(LocalError::Invalid);
                }
                let mut limits = image::Limits::default();
                limits.max_image_width = Some(8192);
                limits.max_image_height = Some(8192);
                limits.max_alloc = Some(128 * 1024 * 1024);
                reader.limits(limits);
                let image = reader.decode().map_err(|_| LocalError::Invalid)?;
                if u64::from(image.width()) * u64::from(image.height()) > 20_000_000 {
                    return Err(LocalError::Invalid);
                }
                let image = image.thumbnail(2400, 2400);
                let mut output = std::io::Cursor::new(Vec::new());
                image
                    .write_to(&mut output, image::ImageFormat::Png)
                    .map_err(|_| LocalError::Invalid)?;
                Ok(output.into_inner())
            })
            .await
            .map_err(|_| LocalError::Unavailable)??;
            if encoded.len() > 10 * 1024 * 1024 {
                return Err(LocalError::Invalid);
            }
            (encoded, "image/png", "png")
        };
        let key = format!("{id}.{extension}");
        let path = self.root.join("assets").join(&key);
        tokio::fs::write(&path, &bytes)
            .await
            .map_err(|_| LocalError::Unavailable)?;
        if is_mp4 || is_webm {
            let result = tokio::time::timeout(
                std::time::Duration::from_secs(10),
                tokio::process::Command::new("ffprobe")
                    .kill_on_drop(true)
                    .args([
                        "-v",
                        "error",
                        "-protocol_whitelist",
                        "file,pipe",
                        "-f",
                        if is_mp4 { "mov" } else { "matroska" },
                        "-show_entries",
                        "format=duration:stream=codec_type,width,height",
                        "-of",
                        "json",
                    ])
                    .arg(&path)
                    .output(),
            )
            .await;
            let valid = match result {
                Ok(Ok(output)) if output.status.success() => {
                    serde_json::from_slice::<serde_json::Value>(&output.stdout)
                        .ok()
                        .is_some_and(|v| {
                            v["format"]["duration"]
                                .as_str()
                                .and_then(|n| n.parse::<f64>().ok())
                                .is_some_and(|d| d > 0.0 && d <= 120.0)
                                && v["streams"].as_array().is_some_and(|streams| {
                                    streams.iter().any(|s| {
                                        s["codec_type"] == "video"
                                            && s["width"].as_u64().is_some_and(|w| w <= 3840)
                                            && s["height"].as_u64().is_some_and(|h| h <= 2160)
                                    })
                                })
                        })
                }
                _ => false,
            };
            if !valid {
                let _ = tokio::fs::remove_file(&path).await;
                return Err(LocalError::Invalid);
            }
        }
        let hash = hex::encode(Sha256::digest(&bytes));
        let result=sqlx::query("INSERT INTO local_assets (id,organization_id,uploaded_by,label,filename,media_type,size_bytes,sha256,storage_key) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)").bind(id).bind(actor.organization_id).bind(actor.id).bind(label.trim()).bind(format!("{}.{}",filename.rsplit_once('.').map(|v|v.0).unwrap_or(filename),extension)).bind(mime).bind(bytes.len() as i64).bind(hash).bind(key).execute(&self.pool).await;
        if result.is_err() {
            let _ = tokio::fs::remove_file(&path).await;
            return Err(LocalError::Unavailable);
        }
        self.audit(
            actor.organization_id,
            Some(actor.id),
            "asset_uploaded",
            &id.to_string(),
        )
        .await?;
        self.asset(actor.organization_id, id).await
    }
    pub async fn delete_asset(&self, actor: &LocalUser, id: Uuid) -> Result<(), LocalError> {
        if !actor.role.may_edit() {
            return Err(LocalError::Forbidden);
        }
        let path = self.asset_path(actor.organization_id, id).await?;
        let used:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM creative_jobs WHERE organization_id=$1 AND proposal::jsonb#>>'{input,asset_id}'=$2 AND state NOT IN ('succeeded','failed','canceled'))").bind(actor.organization_id).bind(id.to_string()).fetch_one(&self.pool).await.map_err(|_|LocalError::Unavailable)?;
        if used {
            return Err(LocalError::Conflict);
        }
        sqlx::query("DELETE FROM local_assets WHERE organization_id=$1 AND id=$2")
            .bind(actor.organization_id)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                if e.as_database_error()
                    .is_some_and(|e| e.is_foreign_key_violation())
                {
                    LocalError::Conflict
                } else {
                    LocalError::Unavailable
                }
            })?;
        let _ = tokio::fs::remove_file(path).await;
        self.audit(
            actor.organization_id,
            Some(actor.id),
            "asset_deleted",
            &id.to_string(),
        )
        .await
    }
}
