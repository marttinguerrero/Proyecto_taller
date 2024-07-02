use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct User {
    name: String,
    mail: String,
}
impl User {
    pub(crate) fn new(name: &str, mail: &str) -> User {
        Self {
            name: name.to_string(),
            mail: mail.to_string(),
        }
    }

    pub fn get_name(&self) -> String {
        self.name.clone()
    }

    pub fn get_mail(&self) -> String {
        self.mail.clone()
    }

    // pub(crate) fn set_name(&mut self, name: &str){
    //     self.name = name.to_string();
    // }

    // pub(crate) fn set_mail(&mut self, mail: &str){
    //     self.mail = Some(mail.to_string());
    // }
}
