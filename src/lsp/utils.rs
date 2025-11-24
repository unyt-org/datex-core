use crate::lsp::LanguageServerBackend;
use crate::lsp::errors::SpannedLSPCompilerError;
use crate::lsp::type_hint_collector::TypeHintCollector;
use datex_core::ast::structs::expression::{
    DatexExpression, DatexExpressionData, List, Map, Statements,
    VariableAccess, VariableAssignment, VariableDeclaration,
};
use datex_core::compiler::error::DetailedCompilerErrors;
use datex_core::compiler::precompiler::precompiled_ast::VariableMetadata;
use datex_core::types::type_container::TypeContainer;
use datex_core::values::core_values::decimal::Decimal;
use datex_core::values::core_values::decimal::typed_decimal::TypedDecimal;
use datex_core::values::core_values::endpoint::Endpoint;
use datex_core::values::core_values::integer::Integer;
use datex_core::values::core_values::integer::typed_integer::TypedInteger;
use datex_core::visitor::VisitAction;
use datex_core::visitor::expression::ExpressionVisitor;
use datex_core::visitor::type_expression::TypeExpressionVisitor;
use realhydroper_lsp::lsp_types::{
    MessageType, Position, Range, TextDocumentPositionParams,
};
use url::Url;

impl LanguageServerBackend {
    pub async fn update_file_contents(&self, url: Url, content: String) {
        let mut compiler_workspace = self.compiler_workspace.borrow_mut();
        let file = compiler_workspace.load_file(url.clone(), content.clone());
        // Clear previous errors for this file
        self.clear_compiler_errors(&url);
        if let Some(errors) = &file.errors {
            self.client
                .log_message(
                    MessageType::ERROR,
                    format!("Failed to compile file {}: {}", url, errors,),
                )
                .await;
            self.collect_compiler_errors(errors, url, &content)
        }
        if let Some(rich_ast) = &file.rich_ast {
            self.client
                .log_message(
                    MessageType::INFO,
                    format!("AST: {:#?}", rich_ast.ast),
                )
                .await;
            self.client
                .log_message(
                    MessageType::INFO,
                    format!("AST metadata: {:#?}", *rich_ast.metadata.borrow()),
                )
                .await;
        }
    }

    pub(crate) fn get_type_hints(
        &self,
        url: Url,
    ) -> Option<Vec<(Position, Option<TypeContainer>)>> {
        let mut workspace = self.compiler_workspace.borrow_mut();
        let file = workspace.get_file_mut(&url).unwrap();
        if let Some(rich_ast) = &mut file.rich_ast {
            let ast = &mut rich_ast.ast;
            let mut collector = TypeHintCollector::default();
            collector.visit_datex_expression(ast);
            Some(
                collector
                    .type_hints
                    .into_iter()
                    .map(|hint| {
                        (
                            self.byte_offset_to_position(hint.0, &file.content)
                                .unwrap(),
                            rich_ast
                                .metadata
                                .borrow()
                                .variables
                                .get(hint.1)
                                .unwrap()
                                .var_type
                                .clone(),
                        )
                    })
                    .collect(),
            )
        } else {
            None
        }
    }

    /// Clears all compiler errors associated with the given file URL.
    fn clear_compiler_errors(&self, url: &Url) {
        let mut spanned_compiler_errors =
            self.spanned_compiler_errors.borrow_mut();
        spanned_compiler_errors.remove(url);
    }

    /// Recursively collects spanned compiler errors into the spanned_compiler_errors field.
    fn collect_compiler_errors(
        &self,
        errors: &DetailedCompilerErrors,
        url: Url,
        file_content: &String,
    ) {
        let mut spanned_compiler_errors =
            self.spanned_compiler_errors.borrow_mut();
        let file_errors =
            spanned_compiler_errors.entry(url.clone()).or_default();

        for error in &errors.errors {
            let span = error
                .span
                .as_ref()
                .map(|span| {
                    self.convert_byte_range_to_document_range(
                        span,
                        file_content,
                    )
                })
                .unwrap_or_else(|| {
                    self.convert_byte_range_to_document_range(
                        &(0..file_content.len()),
                        file_content,
                    )
                });
            file_errors.push(SpannedLSPCompilerError {
                span,
                error: error.error.clone(),
            });
        }
    }

    /// Finds all variables in the workspace whose names start with the given prefix.
    pub fn find_variable_starting_with(
        &self,
        prefix: &str,
    ) -> Vec<VariableMetadata> {
        let compiler_workspace = self.compiler_workspace.borrow();
        let mut results = Vec::new();
        for file in compiler_workspace.files().values() {
            if let Some(rich_ast) = &file.rich_ast {
                let metadata = rich_ast.metadata.borrow();
                for var in metadata.variables.iter() {
                    if var.name.starts_with(prefix) {
                        results.push(var.clone());
                    }
                }
            }
        }
        results
    }

    /// Retrieves variable metadata by its unique ID.
    pub fn get_variable_by_id(&self, id: usize) -> Option<VariableMetadata> {
        let compiler_workspace = self.compiler_workspace.borrow();
        for file in compiler_workspace.files().values() {
            if let Some(rich_ast) = &file.rich_ast {
                let metadata = rich_ast.metadata.borrow();
                if let Some(v) = metadata.variables.get(id).cloned() {
                    return Some(v);
                }
            }
        }
        None
    }

    /// Converts an LSP position (line and character) to a byte offset in the file content.
    fn position_to_byte_offset(
        &self,
        position: &TextDocumentPositionParams,
    ) -> usize {
        let workspace = self.compiler_workspace.borrow();
        // first get file contents at position.text_document.uri
        // then calculate byte offset from position.position.line and position.position.character
        let file_content = &workspace
            .get_file(&position.text_document.uri)
            .unwrap()
            .content;

        Self::line_char_to_byte_index(
            file_content,
            position.position.line as usize,
            position.position.character as usize,
        )
        .unwrap_or(0)
    }

    /// Converts a byte range (start, end) to a document Range (start Position, end Position) in the file content.
    pub fn convert_byte_range_to_document_range(
        &self,
        span: &core::ops::Range<usize>,
        file_content: &String,
    ) -> Range {
        let start = self
            .byte_offset_to_position(span.start, file_content)
            .unwrap_or(Position {
                line: 0,
                character: 0,
            });
        let end = self
            .byte_offset_to_position(span.end, file_content)
            .unwrap_or(Position {
                line: 0,
                character: 0,
            });
        Range { start, end }
    }

    /// Converts a byte offset to an LSP position (line and character) in the file content.
    /// TODO: check if this is correct, generated with copilot
    pub fn byte_offset_to_position(
        &self,
        byte_offset: usize,
        file_content: &String,
    ) -> Option<Position> {
        let mut current_offset = 0;
        for (line_idx, line) in file_content.lines().enumerate() {
            let line_length = line.len() + 1; // +1 for the newline character
            if current_offset + line_length > byte_offset {
                // The byte offset is within this line
                let char_offset = line
                    .char_indices()
                    .find(|(i, _)| current_offset + i >= byte_offset)
                    .map(|(i, _)| i)
                    .unwrap_or(line.len());
                return Some(Position {
                    line: line_idx as u32,
                    character: char_offset as u32,
                });
            }
            current_offset += line_length;
        }
        None
    }

    /// Retrieves the text immediately preceding the given position in the document.
    /// This is used for autocompletion suggestions.
    pub fn get_previous_text_at_position(
        &self,
        position: &TextDocumentPositionParams,
    ) -> String {
        let byte_offset = self.position_to_byte_offset(position);
        let workspace = self.compiler_workspace.borrow();
        let file_content = &workspace
            .get_file(&position.text_document.uri)
            .unwrap()
            .content;
        // Get the text before the byte offset, only matching word characters
        let previous_text = &file_content[..byte_offset];
        let last_word = previous_text
            .rsplit(|c: char| !c.is_alphanumeric() && c != '_')
            .next()
            .unwrap_or("");
        last_word.to_string()
    }

    /// Retrieves the DatexExpression AST node at the given byte offset.
    pub fn get_expression_at_position(
        &self,
        position: &TextDocumentPositionParams,
    ) -> Option<DatexExpression> {
        let byte_offset = self.position_to_byte_offset(position);
        let mut workspace = self.compiler_workspace.borrow_mut();
        if let Some(rich_ast) = &mut workspace
            .get_file_mut(&position.text_document.uri)
            .unwrap()
            .rich_ast
        {
            let ast = &mut rich_ast.ast;
            let mut finder = ExpressionFinder::new(byte_offset);
            finder.visit_datex_expression(ast);
            finder.found_expr.map(|e| DatexExpression {
                span: e.1,
                data: e.0,
                wrapped: None,
                ty: None,
            })
        } else {
            None
        }
    }

    /// Converts a (line, character) pair to a byte index in the given text.
    /// Lines and characters are zero-indexed.
    /// Returns None if the line or character is out of bounds.
    pub fn line_char_to_byte_index(
        text: &str,
        line: usize,
        character: usize,
    ) -> Option<usize> {
        let mut lines = text.split('\n');

        // Get the line
        let line_text = lines.nth(line)?;

        // Compute byte index of the start of that line
        let byte_offset_to_line_start = text
            .lines()
            .take(line)
            .map(|l| l.len() + 1) // +1 for '\n'
            .sum::<usize>();

        // Now find the byte index within that line for the given character offset
        let byte_offset_within_line = line_text
            .char_indices()
            .nth(character)
            .map(|(i, _)| i)
            .unwrap_or_else(|| line_text.len());

        Some(byte_offset_to_line_start + byte_offset_within_line)
    }
}

/// Visitor that finds the most specific DatexExpression containing a given byte position.
/// If multiple expressions contain the position, the one with the smallest span is chosen.
struct ExpressionFinder {
    pub search_pos: usize,
    pub found_expr: Option<(DatexExpressionData, core::ops::Range<usize>)>,
}

impl ExpressionFinder {
    pub fn new(search_pos: usize) -> Self {
        Self {
            search_pos,
            found_expr: None,
        }
    }

    /// Checks if the given span includes the search position.
    /// If it does, updates found_expr if this expression is more specific (smaller span).
    /// Returns true if the span includes the search position, false otherwise.
    fn match_span(
        &mut self,
        span: &core::ops::Range<usize>,
        expr_data: DatexExpressionData,
    ) -> Result<VisitAction<DatexExpression>, ()> {
        if span.start <= self.search_pos && self.search_pos <= span.end {
            // If we already found an expression, only replace it if this one is smaller (more specific)
            if let Some((_, existing_expr_span)) = &self.found_expr {
                if (span.end - span.start)
                    < (existing_expr_span.end - existing_expr_span.start)
                {
                    self.found_expr = Some((expr_data, span.clone()));
                }
            } else {
                self.found_expr = Some((expr_data, span.clone()));
            }
            Ok(VisitAction::VisitChildren)
        } else {
            Ok(VisitAction::SkipChildren)
        }
    }
}

impl TypeExpressionVisitor<()> for ExpressionFinder {}

impl ExpressionVisitor<()> for ExpressionFinder {
    fn visit_statements(
        &mut self,
        stmts: &mut Statements,
        span: &core::ops::Range<usize>,
    ) -> Result<VisitAction<DatexExpression>, ()> {
        self.match_span(span, DatexExpressionData::Statements(stmts.clone()))
    }

    fn visit_variable_declaration(
        &mut self,
        var_decl: &mut VariableDeclaration,
        span: &core::ops::Range<usize>,
    ) -> Result<VisitAction<DatexExpression>, ()> {
        self.match_span(
            span,
            DatexExpressionData::VariableDeclaration(var_decl.clone()),
        )
    }

    fn visit_variable_assignment(
        &mut self,
        var_assign: &mut VariableAssignment,
        span: &core::ops::Range<usize>,
    ) -> Result<VisitAction<DatexExpression>, ()> {
        self.match_span(
            span,
            DatexExpressionData::VariableAssignment(var_assign.clone()),
        )
    }

    fn visit_variable_access(
        &mut self,
        var_access: &mut VariableAccess,
        span: &core::ops::Range<usize>,
    ) -> Result<VisitAction<DatexExpression>, ()> {
        self.match_span(
            span,
            DatexExpressionData::VariableAccess(var_access.clone()),
        )
    }

    fn visit_list(
        &mut self,
        list: &mut List,
        span: &core::ops::Range<usize>,
    ) -> Result<VisitAction<DatexExpression>, ()> {
        self.match_span(span, DatexExpressionData::List(list.clone()))
    }

    fn visit_map(
        &mut self,
        map: &mut Map,
        span: &core::ops::Range<usize>,
    ) -> Result<VisitAction<DatexExpression>, ()> {
        self.match_span(span, DatexExpressionData::Map(map.clone()))
    }

    fn visit_integer(
        &mut self,
        value: &mut Integer,
        span: &core::ops::Range<usize>,
    ) -> Result<VisitAction<DatexExpression>, ()> {
        self.match_span(span, DatexExpressionData::Integer(value.clone()))
    }

    fn visit_typed_integer(
        &mut self,
        value: &mut TypedInteger,
        span: &core::ops::Range<usize>,
    ) -> Result<VisitAction<DatexExpression>, ()> {
        self.match_span(span, DatexExpressionData::TypedInteger(value.clone()))
    }

    fn visit_decimal(
        &mut self,
        value: &mut Decimal,
        span: &core::ops::Range<usize>,
    ) -> Result<VisitAction<DatexExpression>, ()> {
        self.match_span(span, DatexExpressionData::Decimal(value.clone()))
    }

    fn visit_typed_decimal(
        &mut self,
        value: &mut TypedDecimal,
        span: &core::ops::Range<usize>,
    ) -> Result<VisitAction<DatexExpression>, ()> {
        self.match_span(span, DatexExpressionData::TypedDecimal(value.clone()))
    }

    fn visit_text(
        &mut self,
        value: &mut String,
        span: &core::ops::Range<usize>,
    ) -> Result<VisitAction<DatexExpression>, ()> {
        self.match_span(span, DatexExpressionData::Text(value.clone()))
    }

    fn visit_boolean(
        &mut self,
        value: &mut bool,
        span: &core::ops::Range<usize>,
    ) -> Result<VisitAction<DatexExpression>, ()> {
        self.match_span(span, DatexExpressionData::Boolean(*value))
    }

    fn visit_endpoint(
        &mut self,
        value: &mut Endpoint,
        span: &core::ops::Range<usize>,
    ) -> Result<VisitAction<DatexExpression>, ()> {
        self.match_span(span, DatexExpressionData::Endpoint(value.clone()))
    }

    fn visit_null(
        &mut self,
        span: &core::ops::Range<usize>,
    ) -> Result<VisitAction<DatexExpression>, ()> {
        self.match_span(span, DatexExpressionData::Null)
    }
}
