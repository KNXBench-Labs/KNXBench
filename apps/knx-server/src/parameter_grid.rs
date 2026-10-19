//! Projects declared ProductDB parameter grids onto the HTTP panel (ADR-0137).
use crate::routes::{
    ParameterGridCellDto, ParameterGridColumnDto, ParameterGridIssueDto, ParameterGridIssueKindDto,
    ParameterGridLabelDto, ParameterGridRowDto, ParameterGridStatusDto, ParameterGroupLayoutDto,
    PARAMETER_GROUP_LAYOUT_VERSION,
};
use knx_productdb::dynamic::ActiveLabel;
use knx_productdb::parameter_layout::{
    BlockGrid, Cell, GridIssueKind, GridStatus, ParameterLayout,
};

pub(crate) fn cell_dto(cell: Cell) -> ParameterGridCellDto {
    ParameterGridCellDto {
        row: cell.row,
        column: cell.column,
    }
}

/// One group's layout, with the activated separators of exactly this
/// block and scope as captions. Inactive separators never appear.
pub(crate) fn group_layout_dto<'a>(
    layout: &ParameterLayout,
    definition: &str,
    group_key: String,
    block_node: i64,
    captions: &mut crate::parameter_captions::CaptionFills,
    active_labels: impl Iterator<Item = &'a ActiveLabel>,
) -> Result<Option<ParameterGroupLayoutDto>, String> {
    let Some(grid) = layout.grid(definition, block_node) else {
        return Ok(None);
    };
    let mut labels = Vec::new();
    if grid.status == GridStatus::Grid {
        for label in active_labels.filter(|label| label.kind == "ParameterSeparator") {
            let Some(placement) = layout.separator(definition, label.node_id) else {
                continue;
            };
            if placement.block_node != block_node {
                continue;
            }
            let (text, text_fill) = captions.copy(
                (label.scope.as_ref().map(|s| s.node_chain()), label.node_id),
                Some(&label.text),
            )?;
            labels.push(ParameterGridLabelDto {
                row: placement.cell.row,
                column: placement.cell.column,
                text: text.expect("present caption"),
                text_fill,
            });
        }
    }
    grid_dto(group_key, grid, labels, captions).map(Some)
}

fn grid_dto(
    group_key: String,
    grid: &BlockGrid,
    labels: Vec<ParameterGridLabelDto>,
    captions: &mut crate::parameter_captions::CaptionFills,
) -> Result<ParameterGroupLayoutDto, String> {
    Ok(ParameterGroupLayoutDto {
        group_key,
        version: PARAMETER_GROUP_LAYOUT_VERSION,
        declared_layout: grid.declared_layout.clone(),
        status: match grid.status {
            GridStatus::Grid => ParameterGridStatusDto::Grid,
            GridStatus::UnknownLayout => ParameterGridStatusDto::UnknownLayout,
            GridStatus::InvalidGrid => ParameterGridStatusDto::InvalidGrid,
            GridStatus::SourceUnavailable => ParameterGridStatusDto::SourceUnavailable,
        },
        rows: grid
            .rows
            .iter()
            .map(|row| {
                Ok(ParameterGridRowDto {
                    id: row.id.clone(),
                    text: captions.copy_text(row.text.as_deref())?,
                    text_parameter_ref_id: row.text_parameter_ref_id.clone(),
                    collapse_if_empty: row.collapse_if_empty.clone(),
                })
            })
            .collect::<Result<Vec<_>, String>>()?,
        columns: grid
            .columns
            .iter()
            .map(|column| {
                Ok(ParameterGridColumnDto {
                    id: column.id.clone(),
                    text: captions.copy_text(column.text.as_deref())?,
                    text_parameter_ref_id: column.text_parameter_ref_id.clone(),
                    width: column.width.clone(),
                })
            })
            .collect::<Result<Vec<_>, String>>()?,
        labels,
        issues: grid
            .issues
            .iter()
            .map(|issue| ParameterGridIssueDto {
                kind: match issue.kind {
                    GridIssueKind::CellMalformed => ParameterGridIssueKindDto::CellMalformed,
                    GridIssueKind::CellOutOfRange => ParameterGridIssueKindDto::CellOutOfRange,
                    GridIssueKind::MissingCell => ParameterGridIssueKindDto::MissingCell,
                    GridIssueKind::AmbiguousCell => ParameterGridIssueKindDto::AmbiguousCell,
                    GridIssueKind::UnshownContent => ParameterGridIssueKindDto::UnshownContent,
                },
                element: issue.element.clone(),
                reference: issue.reference.clone(),
                cell: issue.cell.clone(),
            })
            .collect(),
    })
}
