use crate::common::GlobalContext;
use crate::moderator::{Data, ModContext};
use axum::body::Body;
use axum::extract::Multipart;
use axum::http::{header, HeaderMap};
use axum::response::{IntoResponse, Response};
use axum::Json;
use object_store::path::Path;
use object_store::{ObjectStore, ObjectStoreExt, PutPayload};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Deserialize)]
pub struct PathQuery {
    p: String,
}
#[derive(Deserialize)]
pub struct RenameRequest {
    p: String,
    new_name: String,
}
#[derive(Deserialize)]
pub struct MoveRequest {
    p: String,
    dir: String,
}

#[derive(Serialize)]
pub struct FileInfo {
    name: String,
    path: String,
    is_dir: bool,
    size: Option<usize>,
    last_modified: Option<String>,
}

type FileStore = GlobalContext<Arc<Box<dyn ObjectStore>>>;

/// 删除文件或目录
pub async fn delete(
    _ctx: ModContext,
    GlobalContext(fs): FileStore,
    Json(req): Json<PathQuery>,
) -> Json<Data<serde_json::Value>> {
    let path = match Path::parse(&req.p) {
        Ok(p) => p,
        Err(e) => return Data::error(&format!("Invalid path: {}", e)),
    };

    match fs.delete(&path).await {
        Ok(_) => Data::ok(serde_json::json!({"deleted": req.p})),
        Err(e) => Data::error(&format!("Failed to delete: {}", e)),
    }
}

/// 上传文件
pub async fn upload(
    _ctx: ModContext,
    GlobalContext(fs): FileStore,
    mut multipart: Multipart,
) -> Json<Data<serde_json::Value>> {
    let mut target_path: Option<String> = None;
    let mut uploaded_files: Vec<String> = Vec::new();

    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or_default().to_string();

        if name == "path" {
            // 获取目标路径
            if let Ok(text) = field.text().await {
                target_path = Some(text);
            }
        } else if name == "file" {
            // 处理文件上传
            let filename = field.file_name().unwrap_or("unnamed").to_string();
            let data = match field.bytes().await {
                Ok(d) => d,
                Err(e) => return Data::error(&format!("Failed to read file data: {}", e)),
            };

            // 构建完整路径
            let base_path = target_path.clone().unwrap_or_default();
            let full_path = if base_path.is_empty() {
                filename.clone()
            } else {
                format!("{}/{}", base_path.trim_end_matches('/'), filename)
            };

            let path = match Path::parse(&full_path) {
                Ok(p) => p,
                Err(e) => return Data::error(&format!("Invalid path: {}", e)),
            };

            let payload = PutPayload::from(data);
            match fs.put(&path, payload).await {
                Ok(_) => uploaded_files.push(full_path),
                Err(e) => return Data::error(&format!("Failed to upload {}: {}", filename, e)),
            }
        }
    }

    if uploaded_files.is_empty() {
        Data::error("No files uploaded")
    } else {
        Data::ok(serde_json::json!({"uploaded": uploaded_files}))
    }
}

/// 列出目录内容
pub async fn list(
    _ctx: ModContext,
    GlobalContext(fs): FileStore,
    Json(req): Json<PathQuery>,
) -> Json<Data<Vec<FileInfo>>> {
    let prefix = if req.p.is_empty() {
        None
    } else {
        match Path::parse(&req.p) {
            Ok(p) => Some(p),
            Err(e) => return Data::error(&format!("Invalid path: {}", e)),
        }
    };

    // 列出目录内容
    let list_result = if let Some(ref p) = prefix {
        fs.list_with_delimiter(Some(p)).await
    } else {
        fs.list_with_delimiter(None).await
    };

    match list_result {
        Ok(result) => {
            let mut files: Vec<FileInfo> = Vec::new();

            // 添加子目录
            for dir in result.common_prefixes {
                let dir_str = dir.to_string();
                let name = dir_str
                    .trim_end_matches('/')
                    .rsplit('/')
                    .next()
                    .unwrap_or(&dir_str)
                    .to_string();
                files.push(FileInfo {
                    name,
                    path: dir_str,
                    is_dir: true,
                    size: None,
                    last_modified: None,
                });
            }

            // 添加文件
            for obj in result.objects {
                let path_str = obj.location.to_string();
                let name = path_str.rsplit('/').next().unwrap_or(&path_str).to_string();
                files.push(FileInfo {
                    name,
                    path: path_str,
                    is_dir: false,
                    size: Some(obj.size as usize),
                    last_modified: Some(obj.last_modified.to_rfc3339()),
                });
            }

            Data::ok(files)
        }
        Err(e) => Data::error(&format!("Failed to list directory: {}", e)),
    }
}

/// 重命名文件
pub async fn rename(
    _ctx: ModContext,
    GlobalContext(fs): FileStore,
    Json(req): Json<RenameRequest>,
) -> Json<Data<serde_json::Value>> {
    let old_path = match Path::parse(&req.p) {
        Ok(p) => p,
        Err(e) => return Data::error(&format!("Invalid source path: {}", e)),
    };

    // 构建新路径：保持目录不变，只改文件名
    let new_path_str = if let Some(parent) = req.p.rsplit_once('/') {
        format!("{}/{}", parent.0, req.new_name)
    } else {
        req.new_name.clone()
    };

    let new_path = match Path::parse(&new_path_str) {
        Ok(p) => p,
        Err(e) => return Data::error(&format!("Invalid target path: {}", e)),
    };

    // 使用 rename 方法
    match fs.rename(&old_path, &new_path).await {
        Ok(_) => Data::ok(serde_json::json!({
            "from": req.p,
            "to": new_path_str
        })),
        Err(e) => Data::error(&format!("Failed to rename: {}", e)),
    }
}

/// 下载文件
pub async fn download(
    _ctx: ModContext,
    GlobalContext(fs): FileStore,
    Json(req): Json<PathQuery>,
) -> Response {
    let path = match Path::parse(&req.p) {
        Ok(p) => p,
        Err(e) => {
            return Data::<()>::fail(&format!("Invalid path: {}", e)).into_response()
        }
    };

    match fs.get(&path).await {
        Ok(result) => {
            let size = result.meta.size;
            let stream = result.into_stream();

            // 获取文件名用于 Content-Disposition
            let filename = req.p.rsplit('/').next().unwrap_or(&req.p);
            let mime = mime_guess::from_path(&req.p).first_or_octet_stream();

            let body = Body::from_stream(stream);

            let mut headers = HeaderMap::new();
            if let Ok(val) = axum::http::HeaderValue::from_str(mime.as_ref()) {
                headers.insert(header::CONTENT_TYPE, val);
            }
            if let Ok(val) = axum::http::HeaderValue::from_str(&size.to_string()) {
                headers.insert(header::CONTENT_LENGTH, val);
            }
            // 设置下载文件名
            let disposition = format!("attachment; filename=\"{}\"", filename);
            if let Ok(val) = axum::http::HeaderValue::from_str(&disposition) {
                headers.insert(header::CONTENT_DISPOSITION, val);
            }

            (headers, body).into_response()
        }
        Err(e) => Data::<()>::fail(&format!("Failed to download: {}", e)).into_response(),
    }
}

/// 移动文件到另一个目录
pub async fn move_file(
    _ctx: ModContext,
    GlobalContext(fs): FileStore,
    Json(req): Json<MoveRequest>,
) -> Json<Data<serde_json::Value>> {
    let old_path = match Path::parse(&req.p) {
        Ok(p) => p,
        Err(e) => return Data::error(&format!("Invalid source path: {}", e)),
    };

    // 获取文件名
    let filename = req.p.rsplit('/').next().unwrap_or(&req.p);

    // 构建目标路径
    let new_path_str = if req.dir.is_empty() {
        filename.to_string()
    } else {
        format!("{}/{}", req.dir.trim_end_matches('/'), filename)
    };

    let new_path = match Path::parse(&new_path_str) {
        Ok(p) => p,
        Err(e) => return Data::error(&format!("Invalid target path: {}", e)),
    };

    // 使用 rename 方法移动文件
    match fs.rename(&old_path, &new_path).await {
        Ok(_) => Data::ok(serde_json::json!({
            "from": req.p,
            "to": new_path_str
        })),
        Err(e) => Data::error(&format!("Failed to move: {}", e)),
    }
}
