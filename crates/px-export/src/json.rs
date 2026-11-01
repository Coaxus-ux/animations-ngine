use std::fmt::Write;

#[derive(Debug, Default)]
pub struct JsonWriter {
    text: String,
    has_items: Vec<bool>,
    after_key: bool,
}

impl JsonWriter {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn begin_object(&mut self) {
        self.open('{');
    }

    pub fn end_object(&mut self) {
        self.close('}');
    }

    pub fn begin_array(&mut self) {
        self.open('[');
    }

    pub fn end_array(&mut self) {
        self.close(']');
    }

    pub fn key(&mut self, key: &str) {
        self.separate();
        self.quote(key);
        self.text.push(':');
        self.after_key = true;
    }

    pub fn string(&mut self, value: &str) {
        self.separate();
        self.quote(value);
    }

    pub fn int(&mut self, value: i64) {
        self.separate();
        let _ = write!(self.text, "{value}");
    }

    pub fn bool(&mut self, value: bool) {
        self.separate();
        self.text.push_str(if value { "true" } else { "false" });
    }

    #[must_use]
    pub fn finish(self) -> String {
        self.text
    }

    fn open(&mut self, bracket: char) {
        self.separate();
        self.text.push(bracket);
        self.has_items.push(false);
    }

    fn close(&mut self, bracket: char) {
        self.has_items.pop();
        self.text.push(bracket);
    }

    fn separate(&mut self) {
        if self.after_key {
            self.after_key = false;
            return;
        }
        if let Some(has_items) = self.has_items.last_mut() {
            if *has_items {
                self.text.push(',');
            }
            *has_items = true;
        }
    }

    fn quote(&mut self, value: &str) {
        self.text.push('"');
        for ch in value.chars() {
            match ch {
                '"' => self.text.push_str("\\\""),
                '\\' => self.text.push_str("\\\\"),
                '\n' => self.text.push_str("\\n"),
                '\t' => self.text.push_str("\\t"),
                '\r' => self.text.push_str("\\r"),
                c if u32::from(c) < 0x20 => {
                    let _ = write!(self.text, "\\u{:04x}", u32::from(c));
                }
                c => self.text.push(c),
            }
        }
        self.text.push('"');
    }
}
