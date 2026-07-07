use std::{fs, io::ErrorKind, path::PathBuf};

use crate::error::{AppError, AppResult};
use crate::models::{find_known_model, ModelStatus};

pub fn app_support_dir() -> AppResult<PathBuf> {
    dirs::data_dir()
        .map(|path| path.join("QwenASR Studio"))
        .ok_or_else(|| AppError::Io("Could not resolve the application support directory".into()))
}

pub fn models_dir() -> AppResult<PathBuf> {
    Ok(app_support_dir()?.join("models"))
}

pub fn model_dir(model_id: &str) -> AppResult<PathBuf> {
    Ok(models_dir()?.join(model_id))
}

pub fn model_status(model_id: &str) -> AppResult<ModelStatus> {
    let model = find_known_model(model_id)
        .ok_or_else(|| AppError::Model(format!("Unknown model: {model_id}")))?;
    let path = model_dir(model.id)?;
    let missing_files = model
        .files
        .iter()
        .filter(|file| !path.join(file).exists())
        .map(|file| (*file).to_string())
        .collect::<Vec<_>>();

    Ok(ModelStatus {
        id: model.id.to_string(),
        title: model.title.to_string(),
        repo: model.repo.to_string(),
        description: model.description.to_string(),
        size_hint: model.size_hint.to_string(),
        recommended: model.recommended,
        installed: missing_files.is_empty(),
        path: path.to_string_lossy().to_string(),
        files: model.files.iter().map(|file| (*file).to_string()).collect(),
        missing_files,
    })
}

pub fn delete_model(model_id: &str) -> AppResult<ModelStatus> {
    let model = find_known_model(model_id)
        .ok_or_else(|| AppError::Model(format!("Unknown model: {model_id}")))?;
    let path = model_dir(model.id)?;

    if !path.exists() {
        return model_status(model.id);
    }

    if !path.is_dir() {
        return Err(AppError::Model(format!(
            "Model path is not a directory: {}",
            path.display()
        )));
    }

    match fs::remove_dir_all(&path) {
        Ok(()) => {}
        Err(error) if error.kind() == ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }

    model_status(model.id)
}
