use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::fs;
use shadow_rs::ShadowBuilder;
use walkdir::WalkDir;
use zip::write::SimpleFileOptions;

fn main() {
    ShadowBuilder::builder().build().unwrap();
    build_locales();
    build_style();
    build_public();
}
fn build_style() {
    // 依然需要监控文件变化
    println!("cargo:rerun-if-changed=templates/scss");

    let scss_file = Path::new("templates/scss/style.scss");
    let css_dest = Path::new("public/style.css");

    // 使用 grass 编译 scss
    let css_content = grass::from_path(scss_file.to_str().unwrap(), &grass::Options::default())
        .expect("Failed to compile SCSS");

    // 确保输出目录存在
    if let Some(parent) = css_dest.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }

    // 写入 CSS 文件
    let mut file = File::create(css_dest).expect("Failed to create CSS file");
    file.write_all(css_content.as_bytes()).unwrap();
}

fn build_public() {
    // 打包 public 目录到 public.zip
    let public_path = Path::new("public");
    let zip_path = Path::new("public.zip");
    let file = File::create(&zip_path).expect("Failed to create zip file");

    let mut zip = zip::ZipWriter::new(file);
    let options = SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Stored)
        .unix_permissions(0o755);

    let walk = WalkDir::new(public_path);
    for entry in walk {
        let entry = entry.unwrap();
        let path = entry.path();

        if path.is_file() {
            println!("cargo:rerun-if-changed={}", path.display());
            let name = path.strip_prefix(public_path).unwrap();
            let name_str = name.to_str().unwrap().replace("\\", "/"); // Normalize path separators

            zip.start_file(name_str, options).unwrap();
            let mut f = File::open(path).unwrap();
            let mut buffer = Vec::new();
            f.read_to_end(&mut buffer).unwrap();
            zip.write_all(&buffer).unwrap();
        }
    }
    zip.finish().unwrap();
}

use toml::{Table, Value};

fn build_locales() {
    // 设置监听目录（按照你的要求写死）
    println!("cargo:rerun-if-changed=locales/en_US");
    println!("cargo:rerun-if-changed=locales/zh_CN");

    let locales_dir = Path::new("locales");

    // 1. 扫描 locales 目录
    if let Ok(entries) = fs::read_dir(locales_dir) {
        for entry in entries.flatten() {
            let path = entry.path();

            // 只处理目录 (例如 en_US, zh_CN)
            if path.is_dir() {
                process_locale_dir(&path, locales_dir);
            }
        }
    }
}

/// 处理单个语言目录 (例如 locales/en_US)
fn process_locale_dir(locale_path: &Path, output_root: &Path) {
    let mut root_table = Table::new();

    // 获取目录名称作为 locale 名称 (例如 en_US)
    let locale_name = match locale_path.file_name().and_then(|n| n.to_str()) {
        Some(name) => name,
        None => return,
    };

    // 2. 扫描子目录下的 .toml 文件
    if let Ok(entries) = fs::read_dir(locale_path) {
        let mut paths: Vec<PathBuf> = entries
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().map_or(false, |ext| ext == "toml"))
            .collect();

        // 排序以确保合并顺序确定（可选，但在某些覆盖场景下有用）
        paths.sort();

        for file_path in paths {
            // 3. 加载并根据文件名合并
            if let Ok(content) = fs::read_to_string(&file_path) {
                match content.parse::<Table>() {
                    Ok(file_table) => {
                        // 获取文件名（不含扩展名），例如 "auth.t"
                        if let Some(stem) = file_path.file_stem().and_then(|s| s.to_str()) {
                            // 5. 依据点号分隔命名空间
                            let keys: Vec<&str> = stem.split('.').collect();
                            merge_into_root(&mut root_table, &keys, file_table);
                        }
                    }
                    Err(e) => {
                        println!("cargo:warning=Failed to parse TOML {:?}: {}", file_path, e);
                    }
                }
            }
        }
    }

    // 4. 保存到 locales 目录下
    let output_filename = format!("{}.toml", locale_name);
    let output_path = output_root.join(output_filename);

    let toml_string = toml::to_string_pretty(&root_table).expect("Failed to serialize TOML");

    // 写入文件
    if let Err(e) = fs::write(&output_path, toml_string) {
        println!("cargo:warning=Failed to write output {:?}: {}", output_path, e);
    }
}

/// 递归地将文件内容合并到根表中
/// keys: 文件名分割后的路径，例如 ["auth", "t"]
/// content: 文件内的 TOML 内容
fn merge_into_root(root: &mut Table, keys: &[&str], content: Table) {
    if keys.is_empty() {
        // 如果文件名为空（理论上不应该发生），直接合并到当前层级
        for (k, v) in content {
            root.insert(k, v);
        }
        return;
    }

    let current_key = keys[0];

    // 如果这是路径的最后一部分 (例如 auth.t 中的 "t")
    if keys.len() == 1 {
        // 找到或创建当前 key 对应的 Table
        let entry = root.entry(current_key.to_string())
            .or_insert_with(|| Value::Table(Table::new()));

        if let Value::Table(target_table) = entry {
            // 将文件内容全部插入到这个 Table 中
            for (k, v) in content {
                target_table.insert(k, v);
            }
        } else {
            panic!("Key conflict: '{}' exists but is not a Table. Check your filenames.", current_key);
        }
    } else {
        // 还有更深层级，例如 auth.t.toml 中的 "auth"
        let entry = root.entry(current_key.to_string())
            .or_insert_with(|| Value::Table(Table::new()));

        if let Value::Table(target_table) = entry {
            // 递归进入下一层
            merge_into_root(target_table, &keys[1..], content);
        } else {
            panic!("Key conflict: '{}' exists but is not a Table. Check your filenames.", current_key);
        }
    }
}
