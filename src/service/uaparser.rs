use crate::util::Renewable;
use anyhow::Result;
use serde::Serialize;
use uaparser::{Parser, UserAgentParser};

/// Result of user agent parsing.
#[derive(Serialize)]
pub struct UserDevice {
    device_family: String,
    device_brand: Option<String>,
    device_model: Option<String>,

    os_family: String,
    os_major: Option<String>,
    os_minor: Option<String>,
    os_patch: Option<String>,
    os_patch_minor: Option<String>,

    agent_family: String,
    agent_major: Option<String>,
    agent_minor: Option<String>,
}

pub struct UaParser(UserAgentParser);

impl Renewable for UaParser {
    fn new(data: Vec<u8>) -> Result<Self> {
        Ok(Self(UserAgentParser::from_bytes(&data)?))
    }
}

impl UaParser {
    pub fn resolve(&self, user_agent: &str) -> UserDevice {
        let parsed = self.0.parse(user_agent);
        UserDevice {
            device_family: parsed.device.family.into(),
            device_brand: parsed.device.brand.map(Into::into),
            device_model: parsed.device.model.map(Into::into),

            os_family: parsed.os.family.into(),
            os_major: parsed.os.major.map(Into::into),
            os_minor: parsed.os.minor.map(Into::into),
            os_patch: parsed.os.patch.map(Into::into),
            os_patch_minor: parsed.os.patch_minor.map(Into::into),

            agent_family: parsed.user_agent.family.into(),
            agent_major: parsed.user_agent.major.map(Into::into),
            agent_minor: parsed.user_agent.minor.map(Into::into),
        }
    }
}
