use handlebars::Handlebars;
use serde_json::json;

use crate::models::{Notification, NotificationData, NotificationError, Result};

/// Template engine for formatting notifications
pub struct NotificationTemplates {
    handlebars: Handlebars<'static>,
}

impl NotificationTemplates {
    pub fn new() -> Result<Self> {
        let mut handlebars = Handlebars::new();
        
        // Register built-in templates
        Self::register_builtin_templates(&mut handlebars)?;
        
        Ok(Self { handlebars })
    }

    fn register_builtin_templates(hb: &mut Handlebars) -> Result<()> {
        // Movie notification templates
        hb.register_template_string(
            "movie_added",
            "🎬 **{{title}}** ({{year}}) has been added to your library"
        ).map_err(|e| NotificationError::TemplateError(e.to_string()))?;

        hb.register_template_string(
            "movie_deleted", 
            "🗑️ **{{title}}** has been removed from your library"
        ).map_err(|e| NotificationError::TemplateError(e.to_string()))?;

        // Download notification templates
        hb.register_template_string(
            "download_started",
            "⬇️ Started downloading **{{movie_title}}** ({{quality}}) from {{indexer}}"
        ).map_err(|e| NotificationError::TemplateError(e.to_string()))?;

        hb.register_template_string(
            "download_completed",
            "✅ **{{movie_title}}** ({{quality}}) download completed! Size: {{size}}"
        ).map_err(|e| NotificationError::TemplateError(e.to_string()))?;

        hb.register_template_string(
            "download_failed",
            "❌ Download failed for **{{movie_title}}** ({{quality}})"
        ).map_err(|e| NotificationError::TemplateError(e.to_string()))?;

        // Import notification templates
        hb.register_template_string(
            "import_completed",
            "📁 **{{movie_title}}** has been imported to your library at {{destination_path}}"
        ).map_err(|e| NotificationError::TemplateError(e.to_string()))?;

        hb.register_template_string(
            "import_failed",
            "❌ Failed to import **{{movie_title}}** - {{error_message}}"
        ).map_err(|e| NotificationError::TemplateError(e.to_string()))?;

        Ok(())
    }

    pub fn render(&self, notification: &Notification) -> Result<String> {
        let template_name = self.get_template_name(&notification.event_type);
        let context = self.create_template_context(notification);

        self.handlebars
            .render(&template_name, &context)
            .map_err(|e| NotificationError::TemplateError(e.to_string()))
    }

    fn get_template_name(&self, event_type: &crate::NotificationEventType) -> String {
        match event_type {
            crate::NotificationEventType::MovieAdded => "movie_added",
            crate::NotificationEventType::MovieDeleted => "movie_deleted",
            crate::NotificationEventType::DownloadStarted => "download_started",
            crate::NotificationEventType::DownloadCompleted => "download_completed",
            crate::NotificationEventType::DownloadFailed => "download_failed",
            crate::NotificationEventType::ImportCompleted => "import_completed",
            crate::NotificationEventType::ImportFailed => "import_failed",
            _ => "default",
        }.to_string()
    }

    fn create_template_context(&self, notification: &Notification) -> serde_json::Value {
        let mut context = json!({
            "title": notification.title,
            "message": notification.message,
            "timestamp": notification.timestamp,
            "event_type": notification.event_type
        });

        // Add specific data based on notification type
        match &notification.data {
            NotificationData::Movie(data) => {
                context["title"] = json!(data.movie.title);
                context["year"] = json!(data.movie.year);
                context["action"] = json!(data.action);
            }
            NotificationData::Download(data) => {
                context["movie_title"] = json!(data.movie_title);
                context["quality"] = json!(data.quality);
                context["size"] = json!(self.format_size(data.size));
                context["indexer"] = json!(data.indexer);
                context["download_client"] = json!(data.download_client);
                context["status"] = json!(data.status);
                if let Some(progress) = data.progress {
                    context["progress"] = json!(format!("{}%", progress));
                }
            }
            NotificationData::Import(data) => {
                context["movie_title"] = json!(data.movie_title);
                context["quality"] = json!(data.quality);
                context["size"] = json!(self.format_size(data.size));
                context["source_path"] = json!(data.source_path);
                context["destination_path"] = json!(data.destination_path);
                context["status"] = json!(data.status);
            }
            NotificationData::Health(data) => {
                context["check_name"] = json!(data.check_name);
                context["status"] = json!(data.status);
                context["error_message"] = json!(data.message);
            }
            NotificationData::Update(data) => {
                context["current_version"] = json!(data.current_version);
                context["new_version"] = json!(data.new_version);
                if let Some(notes) = &data.release_notes {
                    context["release_notes"] = json!(notes);
                }
            }
        }

        context
    }

    fn format_size(&self, bytes: i64) -> String {
        const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
        let mut size = bytes as f64;
        let mut unit_index = 0;

        while size >= 1024.0 && unit_index < UNITS.len() - 1 {
            size /= 1024.0;
            unit_index += 1;
        }

        format!("{:.2} {}", size, UNITS[unit_index])
    }
}