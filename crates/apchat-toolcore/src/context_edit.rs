use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContextEdit {
    DeleteItems { indices: Vec<usize> },
    EditItem { index: usize, new_content: String },
}
