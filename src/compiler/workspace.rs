use std::collections::HashMap;
use std::path::{PathBuf};
use datex_core::compiler::precompiler::AstWithMetadata;
use crate::compiler::error::CompilerError;
use crate::compiler::{parse_datex_script_to_ast, CompileOptions};
use crate::compiler::type_inference::infer_expression_type;
use crate::runtime::Runtime;
use crate::types::type_container::TypeContainer;

/// Represents a file in the compiler workspace with its path, cached content and AST.
pub struct WorkspaceFile {
    pub path: PathBuf,
    pub content: String,
    pub ast_with_metadata: AstWithMetadata,
    pub return_type: TypeContainer
}


/// Represents the compiler workspace containing multiple files.
#[derive(Default)]
pub struct CompilerWorkspace {
    files: HashMap<PathBuf, WorkspaceFile>,
    runtime: Runtime
}


impl CompilerWorkspace {
    /// Creates a new compiler workspace with the given runtime.
    pub fn new(runtime: Runtime) -> Self {
        Self {
            files: HashMap::new(),
            runtime
        }
    }
    
    pub fn files(&self) -> &HashMap<PathBuf, WorkspaceFile> {
        &self.files
    }

    /// Loads a file into the workspace, caching its content and AST.
    /// Returns a compiler error if parsing or precompilation fails.
    pub fn load_file(&mut self, path: PathBuf, content: String) -> Result<&WorkspaceFile, CompilerError> {
        let (ast_with_metadata, return_type) = self.get_ast_with_metadata_for_file(&path, content.clone())?;
        let workspace_file = WorkspaceFile {
            path: path.clone(),
            content,
            ast_with_metadata,
            return_type
        };
        self.files.insert(path.clone(), workspace_file);
        Ok(self.files.get(&path).unwrap())
    }

    /// Retrieves a reference to a workspace file by its path.
    pub fn get_file(&self, path: &PathBuf) -> Option<&WorkspaceFile> {
        self.files.get(path)
    }

    /// Retrieves the AST with metadata for a given file path and content after parsing and compilation.
    /// Returns a compiler error if parsing or compilation fails.
    fn get_ast_with_metadata_for_file(&self, path: &PathBuf, content: String) -> Result<(AstWithMetadata, TypeContainer), CompilerError> {
        let mut options = CompileOptions::default();
        let mut ast_with_metadata = parse_datex_script_to_ast(&content, &mut options)?;
        let return_type = infer_expression_type(&mut ast_with_metadata.ast.as_mut().unwrap(), ast_with_metadata.metadata.clone())?;
        Ok((
            ast_with_metadata,
            return_type
        ))
    }
}
