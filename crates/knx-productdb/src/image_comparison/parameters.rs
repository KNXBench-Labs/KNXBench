//! Projects captured parameter fields through exactly the memory ranges actually compared.

use crate::image::{AttributedValue, DownloadImage, ParameterAttribution};
use crate::image_comparison::ComparisonRow;
use knx_core::commissioning::parameter_image::read_number;
use std::collections::BTreeMap;

pub(crate) fn rows(
    image: &DownloadImage,
    project: &BTreeMap<u32, u8>,
    device: &BTreeMap<u32, u8>,
) -> Vec<ComparisonRow> {
    image
        .parameter_fields
        .iter()
        .filter_map(|parameter| {
            let segment = image.segment(&parameter.segment)?;
            let address = segment.address.checked_add(parameter.field.offset)?;
            let length =
                (u32::from(parameter.field.bit_offset) + parameter.field.size_in_bit).div_ceil(8);
            if address > u32::from(u16::MAX)
                || address.checked_add(length)? > u32::from(u16::MAX) + 1
            {
                return None;
            }
            let covering = |data: &BTreeMap<u32, u8>| -> Option<Vec<u8>> {
                if parameter.field.offset.checked_add(length)? as usize > segment.octets.len() {
                    return None;
                }
                (0..length)
                    .map(|i| data.get(&(address + i)).copied())
                    .collect()
            };
            let planned = covering(project)
                .as_deref()
                .and_then(|bytes| value(parameter, bytes));
            let expected = match &parameter.value {
                AttributedValue::Number { value, .. } => value.to_string(),
                AttributedValue::Text { value, .. } => value.clone(),
            };
            // Attribution was captured at the write; verify it still agrees with the
            // final plan, so later layout writes cannot silently repurpose its bits.
            let project = planned.filter(|value| *value == expected);
            let device = if project.is_some() {
                covering(device)
                    .as_deref()
                    .and_then(|bytes| value(parameter, bytes))
            } else {
                None
            };
            Some(ComparisonRow {
                kind: "parameter",
                parameter: Some(parameter.clone()),
                label: format!(
                    "{} · {} · bit {} / {}",
                    parameter.name,
                    parameter.reference,
                    parameter.field.bit_offset,
                    parameter.field.size_in_bit
                ),
                address,
                reason: (project.is_none() || device.is_none())
                    .then(|| "parameterNotDecoded".into()),
                project,
                device,
            })
        })
        .collect()
}

fn value(parameter: &ParameterAttribution, bytes: &[u8]) -> Option<String> {
    match &parameter.value {
        AttributedValue::Number {
            nonnegative_signed, ..
        } => {
            let number = read_number(parameter.field, bytes)?;
            if *nonnegative_signed && number >> (parameter.field.size_in_bit - 1) != 0 {
                return None;
            }
            Some(number.to_string())
        }
        AttributedValue::Text { encoding, .. } => {
            if parameter.field.bit_offset != 0
                || parameter.field.size_in_bit == 0
                || !parameter.field.size_in_bit.is_multiple_of(8)
                || bytes.len() != (parameter.field.size_in_bit / 8) as usize
            {
                return None;
            }
            crate::image::decode_parameter_text(encoding.as_deref(), bytes)
        }
    }
}
