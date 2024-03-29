use std::borrow::Cow;

use crate::lint::{fixes_for_diagnostics, DiagnosticFix};
use crate::server::api::LSPResult;
use crate::server::SupportedCodeActionKind;
use crate::server::{client::Notifier, Result};
use crate::session::DocumentSnapshot;
use crate::PositionEncoding;
use lsp_server::ErrorCode;
use lsp_types::{self as types, request as req};
use ruff_linter::settings::LinterSettings;
use types::WorkspaceEdit;

pub(crate) struct CodeActionResolve;

impl super::RequestHandler for CodeActionResolve {
    type RequestType = req::CodeActionResolveRequest;
}

impl super::BackgroundDocumentRequestHandler for CodeActionResolve {
    fn document_url(params: &types::CodeAction) -> Cow<types::Url> {
        let uri: lsp_types::Url = serde_json::from_value(params.data.clone().unwrap_or_default())
            .expect("code actions should have a URI in their data fields");
        std::borrow::Cow::Owned(uri)
    }
    fn run_with_snapshot(
        snapshot: DocumentSnapshot,
        _notifier: Notifier,
        action: types::CodeAction,
    ) -> Result<types::CodeAction> {
        let document = snapshot.document();

        let action_kind: SupportedCodeActionKind = action
            .kind
            .clone()
            .ok_or(anyhow::anyhow!("No kind was given for code action"))
            .with_failure_code(ErrorCode::InvalidParams)?
            .try_into()
            .map_err(|()| anyhow::anyhow!("Code action was of an invalid kind"))
            .with_failure_code(ErrorCode::InvalidParams)?;

        match action_kind {
            SupportedCodeActionKind::SourceFixAll => resolve_edit_for_fix_all(
                action,
                document,
                snapshot.url(),
                &snapshot.configuration().linter,
                snapshot.encoding(),
            )
            .with_failure_code(ErrorCode::InternalError),
            SupportedCodeActionKind::SourceOrganizeImports => resolve_edit_for_organize_imports(
                action,
                document,
                snapshot.url(),
                snapshot.configuration().linter.clone(),
                snapshot.encoding(),
            )
            .with_failure_code(ErrorCode::InternalError),
            SupportedCodeActionKind::QuickFix => Err(anyhow::anyhow!(
                "Got a code action that should not need additional resolution: {action_kind:?}"
            ))
            .with_failure_code(ErrorCode::InvalidParams),
        }
    }
}

pub(super) fn resolve_edit_for_fix_all(
    mut action: types::CodeAction,
    document: &crate::edit::Document,
    url: &types::Url,
    linter_settings: &LinterSettings,
    encoding: PositionEncoding,
) -> crate::Result<types::CodeAction> {
    let diagnostics = crate::lint::check(document, linter_settings, encoding);

    let fixes = fixes_for_diagnostics(document, url, encoding, document.version(), diagnostics)?;

    action.edit = fix_all_edit(fixes.as_slice());

    Ok(action)
}

pub(super) fn resolve_edit_for_organize_imports(
    mut action: types::CodeAction,
    document: &crate::edit::Document,
    url: &types::Url,
    mut linter_settings: ruff_linter::settings::LinterSettings,
    encoding: PositionEncoding,
) -> crate::Result<types::CodeAction> {
    linter_settings.rules = [
        ruff_linter::registry::Rule::from_code("I001").unwrap(),
        ruff_linter::registry::Rule::from_code("I002").unwrap(),
    ]
    .into_iter()
    .collect();

    let diagnostics = crate::lint::check(document, &linter_settings, encoding);

    let fixes = crate::lint::fixes_for_diagnostics(
        document,
        url,
        encoding,
        document.version(),
        diagnostics,
    )?;

    action.edit = Some(types::WorkspaceEdit {
        document_changes: Some(types::DocumentChanges::Edits(
            fixes
                .into_iter()
                .flat_map(|fix| fix.document_edits.into_iter())
                .collect(),
        )),
        ..Default::default()
    });

    Ok(action)
}

fn fix_all_edit(fixes: &[DiagnosticFix]) -> Option<WorkspaceEdit> {
    let edits_made: Vec<_> = fixes
        .iter()
        .filter(|fix| fix.applicability.is_safe())
        .collect();

    if edits_made.is_empty() {
        return None;
    }

    Some(types::WorkspaceEdit {
        document_changes: Some(types::DocumentChanges::Edits(
            edits_made
                .into_iter()
                .flat_map(|fixes| fixes.document_edits.iter())
                .cloned()
                .collect(),
        )),
        ..Default::default()
    })
}
