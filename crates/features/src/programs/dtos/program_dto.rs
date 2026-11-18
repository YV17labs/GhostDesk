use nest_rs::core::output;
use platform::desktop::DesktopApp;

#[output]
#[derive(Debug, Clone)]
pub struct ProgramDto {
    pub name: String,
    pub exec: String,
}

impl From<DesktopApp> for ProgramDto {
    fn from(app: DesktopApp) -> Self {
        Self {
            name: app.name,
            exec: app.exec,
        }
    }
}
