use super::{Event, EventValue};
use crate::StreamId;
use chrono::{DateTime, Utc};

pub enum VerifyOption {
    ResourceModelsContain(StreamId),
    ExpirationTimeBefore(DateTime<Utc>),
}

impl Event {
    pub fn verify_signature(
        &self,
        opts: Vec<VerifyOption>,
    ) -> anyhow::Result<Option<DateTime<Utc>>> {
        let mut expiration_time = None;
        if let EventValue::Signed(signed) = &self.value {
            if let Some(cacao) = signed.cacao()? {
                if signed.cap()? != signed.cacao_link()? {
                    anyhow::bail!("cacao not match jws cap");
                }
                for ele in opts {
                    match ele {
                        VerifyOption::ResourceModelsContain(model) => {
                            let resource_models = cacao.p.resource_models()?;
                            if !resource_models.contains(&model) {
                                anyhow::bail!("invalid resource model");
                            }
                        }
                        VerifyOption::ExpirationTimeBefore(before) => {
                            expiration_time = cacao.p.expiration_time()?;
                            if let Some(exp) = expiration_time {
                                if exp < before {
                                    anyhow::bail!("jws commit expired");
                                }
                            }
                        }
                    }
                }
            };
        };
        Ok(expiration_time)
    }
}
