//! cEMI `L_Data` frame decode (EMI_IMI v01.04.02 AS §4.1.4, §4.1.5.3).
//! Interprets the group-communication trio (`A_GroupValue_Read/Response/
//! Write`, Application Layer v02.01.01 AS §2.2 Table 1), the connection-
//! oriented `A_DeviceDescriptor_Read/Response` pair (§3.4.2.1), and every
//! `Tpci` a `T_Connect`/`T_Data_Connected`/`T_Disconnect` exchange needs
//! (Transport Layer v01.02.03 AS §2, Figure 3 — see `Tpci` below); every
//! other APCI is preserved as raw bytes (`ApplicationService::Other`),
//! never silently dropped.

use knx_core::commissioning::domain_address::DomainAddress;
use knx_core::{GroupAddress, IndividualAddress};
// Re-exported (not just imported) so `crate::cemi::GroupValue` keeps
// resolving for call sites that named this module directly — `GroupValue`
// itself now lives in `knx-core` (spec E4-D2).
pub use knx_core::GroupValue;

pub const L_DATA_REQ: u8 = 0x11;
pub const L_DATA_CON: u8 = 0x2E;
pub const L_DATA_IND: u8 = 0x29;

/// The APCIs of the management services this module types, read from
/// Application Layer v02.01.01 AS Table 1 (spec §6.6, `[D]`). They are
/// full 10-bit values: the memory trio's low six bits carry `number`,
/// `A_Restart`'s carry the response bit and the Restart Type, and the
/// `1111_xxxxxx` family uses all ten bits to name the service.
pub const APCI_INDIVIDUAL_ADDRESS_WRITE: u16 = 0x0C0;
/// `A_IndividualAddress_Read-PDU` (`0100000000`), broadcast; spec §4.2.
pub const APCI_INDIVIDUAL_ADDRESS_READ: u16 = 0x100;
/// `A_IndividualAddress_Response-PDU` (`0101000000`). Carries no data at
/// all — the answering device is named by the frame's source address
/// (spec §4.2), which is why this variant has no fields.
pub const APCI_INDIVIDUAL_ADDRESS_RESPONSE: u16 = 0x140;
/// `A_IndividualAddressSerialNumber_Read-PDU` (`1111011100`), broadcast.
/// AL §3.2.4, Figure 12, p. 21: six octets of KNX Serial Number follow.
pub const APCI_IA_SERIAL_NUMBER_READ: u16 = 0x3DC;
/// `A_IndividualAddressSerialNumber_Response-PDU` (`1111011101`). AL
/// Figure 13, p. 22: serial number (6), domain address (2), reserved (2).
/// The answering device's individual address is the frame's source.
pub const APCI_IA_SERIAL_NUMBER_RESPONSE: u16 = 0x3DD;
/// `A_IndividualAddressSerialNumber_Write-PDU` (`1111011110`). AL §3.2.5,
/// Figure 14, p. 23: serial number (6), new address (2), reserved (4).
pub const APCI_IA_SERIAL_NUMBER_WRITE: u16 = 0x3DE;
/// `A_DomainAddress_Write-PDU` (`1111100000`). AL §3.3.3, Figures 20/21,
/// p. 34: two octets (PL110) or six (RF) of domain address follow.
pub const APCI_DOMAIN_ADDRESS_WRITE: u16 = 0x3E0;
/// `A_DomainAddress_Read-PDU` (`1111100001`). AL §3.3.4, Figure 22: no data.
pub const APCI_DOMAIN_ADDRESS_READ: u16 = 0x3E1;
/// `A_DomainAddress_Response-PDU` (`1111100010`). AL Figures 23/24.
pub const APCI_DOMAIN_ADDRESS_RESPONSE: u16 = 0x3E2;
/// `A_DomainAddressSerialNumber_Read-PDU` (`1111101100`). AL §3.3.6,
/// Figure 26: six octets of KNX Serial Number.
pub const APCI_DOA_SERIAL_NUMBER_READ: u16 = 0x3EC;
/// `A_DomainAddressSerialNumber_Response-PDU` (`1111101101`). AL Figures
/// 27/28: serial number (6), then the domain address (2 or 6).
pub const APCI_DOA_SERIAL_NUMBER_RESPONSE: u16 = 0x3ED;
/// `A_DomainAddressSerialNumber_Write-PDU` (`1111101110`). AL §3.3.7,
/// Figures 29/30: serial number (6), then the new domain address (2 or 6).
/// The KNX IP forms of Figures 31/32 (4 and 21 octets) are not decoded.
pub const APCI_DOA_SERIAL_NUMBER_WRITE: u16 = 0x3EE;
/// `A_Memory_Read-PDU` (`1000nnnnnn`), `number` in the low six bits.
pub const APCI_MEMORY_READ: u16 = 0x200;
/// `A_Memory_Response-PDU` (`1001nnnnnn`).
pub const APCI_MEMORY_RESPONSE: u16 = 0x240;
/// `A_Memory_Write-PDU` (`1010nnnnnn`).
pub const APCI_MEMORY_WRITE: u16 = 0x280;
/// `A_UserMemory_Read-PDU` (`1011000000`).
pub const APCI_USER_MEMORY_READ: u16 = 0x2C0;
/// `A_UserMemory_Response-PDU` (`1011000001`).
pub const APCI_USER_MEMORY_RESPONSE: u16 = 0x2C1;
/// `A_UserMemory_Write-PDU` (`1011000010`).
pub const APCI_USER_MEMORY_WRITE: u16 = 0x2C2;
/// `A_Restart-PDU` (`1110000000`), low six bits per spec §8.
pub const APCI_RESTART: u16 = 0x380;
/// `A_Authorize_Request-PDU` (`1111010001`).
pub const APCI_AUTHORIZE_REQUEST: u16 = 0x3D1;
/// `A_Authorize_Response-PDU` (`1111010010`).
pub const APCI_AUTHORIZE_RESPONSE: u16 = 0x3D2;
/// `A_Key_Write-PDU` (`1111010011`) — named so the value is reserved and
/// not reused, deliberately without a variant: spec §10.9 puts key writing
/// out of phase 2, and a service with no encoder cannot be sent by
/// accident.
pub const APCI_KEY_WRITE: u16 = 0x3D3;
/// `A_PropertyValue_Read-PDU` (`1111010101`).
pub const APCI_PROPERTY_VALUE_READ: u16 = 0x3D5;
/// `A_PropertyValue_Response-PDU` (`1111010110`).
pub const APCI_PROPERTY_VALUE_RESPONSE: u16 = 0x3D6;
/// `A_PropertyValue_Write-PDU` (`1111010111`).
pub const APCI_PROPERTY_VALUE_WRITE: u16 = 0x3D7;

/// The four-bit selector shared by services whose low six bits are a data
/// field (the memory trio, `A_Restart`).
const APCI_SELECTOR_MASK: u16 = 0x3C0;
/// `A_Memory_*`'s `number` field is six bits: 1 to 63 octets
/// (Application Layer v02.01.01 AS §3.4.4, Figures 74-76).
pub const MEMORY_MAX_OCTETS: u8 = 63;
/// `A_UserMemory_*`'s `number` field is four bits: 1 to 15 octets, and its
/// address is 20 bits — *"4 bit address extension + 8 bit address high +
/// 8 bit address low"* (Application Layer v02.01.01 AS §3.5.6.2/§3.5.6.3,
/// Figures 79-81), `[D]`. This is the cap spec §6.4's chunk arithmetic
/// does not mention, because §6.4 is written for `A_Memory_Write`.
pub const USER_MEMORY_MAX_OCTETS: u8 = 15;
/// The top of the 20-bit space `A_UserMemory_*` can address.
pub const USER_MEMORY_ADDRESS_LIMIT: u32 = 1 << 20;
/// `A_Restart`'s Response bit (bit 5 of the APCI's low six).
const RESTART_RESPONSE_BIT: u8 = 0x20;
/// `A_Restart`'s reserved bits 4-1, which are zero in a well-formed PDU.
const RESTART_RESERVED_BITS: u8 = 0x1E;
/// `A_PropertyValue_*`'s `nr_of_elem` is four bits and `start_index`
/// twelve, sharing two octets (spec §7.3; AL §3.4.6 Figures 84/85).
pub const PROPERTY_MAX_NR_OF_ELEM: u8 = 15;
/// The twelve-bit ceiling of `start_index`.
pub const PROPERTY_MAX_START_INDEX: u16 = 0x0FFF;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LDataMessageKind {
    Request,
    /// `error` is Ctrl1's Confirm flag (EMI_IMI v01.04.02 AS §4.1.5.3.4):
    /// `false` = no error, `true` = error.
    Confirmation {
        error: bool,
    },
    Indication,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Destination {
    Individual(IndividualAddress),
    Group(GroupAddress),
    /// The system broadcast: group address `0000h` with cEMI Ctrl1's SB
    /// flag at `0` (EMI_IMI v01.04.02 AS §4.1.5.3.2, p. 76: *"0: system
    /// broadcast, 1: broadcast"*). DLL General §2.3: *"The Destination
    /// Address shall be the system broadcast address (Domain Address =
    /// 0000h and destination address = 0000h and address_type =
    /// multicast)"*. The domain-address services travel on it (AL §3.3).
    /// On RF, SB also says the frame carries a KNX Serial Number rather
    /// than a Domain Address (EMI_IMI §4.1.4.3.9).
    SystemBroadcast,
}

/// The broadcast destination `0/0/0`, used by `A_IndividualAddress_Read` and
/// `A_IndividualAddress_Write` (spec §4.2).
///
/// `[D]` The value is the Transport Layer's, not a convention of this
/// crate: `03_03_04 Transport Layer v01.02.03 AS`, clause 2 "TPDU",
/// Figure 3 — Transport Control Field, page 6 of 38, distinguishes the two
/// unnumbered group-addressed PDUs by destination address alone —
/// *"T_Data_Broadcast-PDU (destination_address = 0)"* against
/// *"T_Data_Group-PDU (destination_address <> 0)"*. Group address 0 is
/// therefore not merely reserved: it is what makes a frame a broadcast.
///
/// `broadcast_destination_is_group_address_zero` pins the number, because
/// the client and the simulator both read this constant and would agree
/// with each other on any wrong value.
pub const BROADCAST_DESTINATION: Destination = Destination::Group(GroupAddress::from_raw(0));

/// The system broadcast (see [`Destination::SystemBroadcast`]).
pub const SYSTEM_BROADCAST_DESTINATION: Destination = Destination::SystemBroadcast;

/// cEMI Ctrl1 bit 4, System Broadcast (EMI_IMI v01.04.02 AS §4.1.5.3.2):
/// set means *broadcast*, clear means *system broadcast*. Every frame this
/// crate sent before K16 had it set (Ctrl1 `0xBC`, `0xB2`, `0xB0`).
const CTRL1_SB_BROADCAST: u8 = 0x10;

/// Octet 6 of the `L_Data` frame (the TPDU's Transport Control Field).
/// Bit layout `[D]`: `03_03_04 Transport Layer v01.02.03 AS`, clause 2
/// "TPDU", Figure 3 — Transport Control Field, page 6 of 38 (the Markdown
/// extraction destroys the figure; verified against the PDF on
/// 2026-09-12). Bit 7 is the Data/Control flag, bit 6 is Numbered:
///
/// | PDU | Octet 6 | Value |
/// | --- | --- | --- |
/// | `T_Data_Broadcast`/`T_Data_Group`/`T_Data_Individual` | `0000 0000` | `0x00` |
/// | `T_Data_Connected` | `01 SeqNo SeqNo SeqNo SeqNo 00` | `0x40 \| seq << 2` |
/// | `T_Connect` | `1000 0000` | `0x80` |
/// | `T_Disconnect` | `1000 0001` | `0x81` |
/// | `T_ACK` | `11 SeqNo SeqNo SeqNo SeqNo 10` | `0xC2 \| seq << 2` |
/// | `T_NAK` | `11 SeqNo SeqNo SeqNo SeqNo 11` | `0xC3 \| seq << 2` |
///
/// NOTE 1 under the figure reserves `0xBF`. `T_Data_Tag_Group` (`0x04`),
/// `0xBF`, and every other pattern none of the named variants match decode
/// as `Unknown(octet)` — the raw octet preserved, not folded into
/// `UnnumberedData` or any other named variant just because some bits
/// happen to match (Global Constraint 2; mirrors `ApplicationService::
/// Other`'s treatment of an unrecognized APCI).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tpci {
    /// TPCI 0b00xxxxxx — what every group service, and every unconnected
    /// point-to-point service, uses.
    UnnumberedData,
    /// TPCI 0b01ssssxx, `seq` 0-15 — a connection-oriented, sequenced
    /// data PDU (`T_Data_Connected`).
    NumberedData { seq: u8 },
    /// TPCI 0b10000000 (`T_Connect`).
    Connect,
    /// TPCI 0b10000001 (`T_Disconnect`).
    Disconnect,
    /// TPCI 0b11ssss10, `seq` 0-15 (`T_ACK`).
    Ack { seq: u8 },
    /// TPCI 0b11ssss11, `seq` 0-15 (`T_NAK`).
    Nak { seq: u8 },
    /// A Transport Control Field octet none of the above match —
    /// `T_Data_Tag_Group` (`0x04`), NOTE 1's reserved `0xBF`, or any other
    /// unrecognized bit pattern. Preserved raw rather than discarded.
    /// Whether the TPDU that carries it also has an APCI octet is decided
    /// the same way the Standard decides it for every named variant: bit 7
    /// (Data/Control) — clear means data-shaped (an APCI follows, handled
    /// like `UnnumberedData`), set means control-shaped (the TPDU is this
    /// octet alone, handled like `Connect`/`Disconnect`/`Ack`/`Nak`). See
    /// `is_control_pdu`.
    ///
    /// **Data-shaped values only ever carry bits 7-2.** Bits 1-0 of a
    /// data-shaped TPCI octet are the short-APCI's top two bits (see
    /// `decode_l_data`'s `short_apci` computation) — they belong to
    /// `ApplicationService`, not to transport, and this variant's whole
    /// purpose is to hold exactly the bits none of the named `Tpci`
    /// variants claim. Both `decode_tpci` and `encode_tpci` mask them to 0
    /// for a data-shaped octet rather than storing/replaying whatever a
    /// caller (or the wire) put there: masking here is not discarding
    /// information (Global Constraint 2's usual concern) because those two
    /// bits are never transport's to begin with — the encoder already
    /// writes them from `service`. A control-shaped octet (bit 7 set) has
    /// no such foreign field sharing it, so it keeps all eight bits.
    Unknown(u8),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApplicationService {
    GroupValueRead,
    GroupValueResponse(GroupValue),
    GroupValueWrite(GroupValue),
    /// `A_DeviceDescriptor_Read-PDU` (`03_03_07 Application Layer v02.01.01
    /// AS`, §3.4.2.1 "A_DeviceDescriptor_Read-service", Figure 36, page 48
    /// of 191): APCI `0x300 | descriptor_type`, `[D]`.
    DeviceDescriptorRead {
        descriptor_type: u8,
    },
    /// `A_DeviceDescriptor_Response-PDU` (same clause/table): APCI
    /// `0x340 | descriptor_type`, `[D]`. `data` is the descriptor value
    /// octets that follow the APCI.
    ///
    /// `[D]` **Known collision, not a bug**: `A_DeviceDescriptor_InfoReport
    /// -PDU` uses this exact same 4-bit APCI selector and the exact same
    /// octet layout (Application Layer v02.01.01 AS §3.3.2 NOTE 5: "This
    /// service uses the same 4 bit APCI as APCI_DeviceDescriptor_Response",
    /// and Figure 19). The two PDUs are byte-identical at this layer; only
    /// the transport service they travel on distinguishes them
    /// (`T_Data_SystemBroadcast` for InfoReport, `T_Data_Individual` for
    /// Response) — a distinction this decoder does not have the KNXnet/IP
    /// framing to make. An `A_DeviceDescriptor_InfoReport` will therefore
    /// decode as this variant.
    DeviceDescriptorResponse {
        descriptor_type: u8,
        data: Vec<u8>,
    },
    /// `A_IndividualAddress_Write-PDU` (AL Table 1 `0011000000`, spec
    /// §4.1): broadcast, two data octets carrying the new address.
    IndividualAddressWrite {
        address: IndividualAddress,
    },
    /// `A_IndividualAddress_Read-PDU` (spec §4.2 step 1): broadcast, no
    /// data. Every device in programming mode answers.
    IndividualAddressRead,
    /// `A_IndividualAddress_Response-PDU`: no data octets. The address
    /// being reported is the frame's `source` — reading it from anywhere
    /// else is the mistake this fieldless variant makes impossible.
    IndividualAddressResponse,
    /// `A_IndividualAddressSerialNumber_Read-PDU` (AL §3.2.4): broadcast.
    /// Only the device with this KNX Serial Number answers.
    IndividualAddressSerialNumberRead {
        serial_number: [u8; 6],
    },
    /// `A_IndividualAddressSerialNumber_Response-PDU` (AL Figure 13). The
    /// individual address is the frame's `source` (MP §2.4, p. 16:
    /// *"The Individual Address is contained as the Source Address"*).
    /// `domain_address` is meaningful on open media only; the two
    /// reserved octets are not kept.
    IndividualAddressSerialNumberResponse {
        serial_number: [u8; 6],
        domain_address: u16,
    },
    /// `A_IndividualAddressSerialNumber_Write-PDU` (AL §3.2.5): broadcast;
    /// the device with this serial number takes `address`, programming
    /// mode or not. The four reserved octets go out as zero.
    IndividualAddressSerialNumberWrite {
        serial_number: [u8; 6],
        address: IndividualAddress,
    },
    /// `A_DomainAddress_Write-PDU` (AL §3.3.3): system broadcast; only a
    /// device in programming mode takes it.
    DomainAddressWrite {
        domain_address: DomainAddress,
    },
    /// `A_DomainAddress_Read-PDU` (AL §3.3.4): system broadcast; every
    /// device in programming mode answers.
    DomainAddressRead,
    /// `A_DomainAddress_Response-PDU`. The answering device's individual
    /// address is the frame's source (MP §2.7: *"source_address = IAn"*).
    DomainAddressResponse {
        domain_address: DomainAddress,
    },
    /// `A_DomainAddressSerialNumber_Read-PDU` (AL §3.3.6).
    DomainAddressSerialNumberRead {
        serial_number: [u8; 6],
    },
    /// `A_DomainAddressSerialNumber_Response-PDU` (AL Figures 27/28).
    DomainAddressSerialNumberResponse {
        serial_number: [u8; 6],
        domain_address: DomainAddress,
    },
    /// `A_DomainAddressSerialNumber_Write-PDU` (AL §3.3.7, Figures 29/30):
    /// the device with this serial number takes the domain address,
    /// programming mode or not.
    DomainAddressSerialNumberWrite {
        serial_number: [u8; 6],
        domain_address: DomainAddress,
    },
    /// `A_Memory_Read-PDU`: `number` octets from `address` (AL §3.4.4
    /// Figure 74). `number` is the request's own field, so it is explicit
    /// here; 1 to 63 on encode.
    MemoryRead {
        number: u8,
        address: u16,
    },
    /// `A_Memory_Response-PDU` (Figure 75). `number` is *not* a field:
    /// it is `data.len()`, so the two can never disagree. An empty `data`
    /// is therefore the documented failure answer — *"the parameter number
    /// of the A_Memory_Response-PDU shall be zero and shall contain no
    /// data"* (spec §6.1) — and not a shape this type has to police.
    MemoryResponse {
        address: u16,
        data: Vec<u8>,
    },
    /// `A_Memory_Write-PDU` (Figure 76), `number` again derived from
    /// `data.len()`. Unlike a response, an empty write has no meaning, so
    /// encoding one is an error rather than a zero-length frame.
    MemoryWrite {
        address: u16,
        data: Vec<u8>,
    },
    /// `A_UserMemory_Read-PDU` (AL §3.5.6.2 Figure 79): a 20-bit
    /// `address` and a four-bit `number`, for the memory above `FFFFh`
    /// spec §6.5 routes here.
    UserMemoryRead {
        number: u8,
        address: u32,
    },
    /// `A_UserMemory_Response-PDU` (Figure 80); `number` is `data.len()`,
    /// zero meaning the same documented failure as `MemoryResponse`.
    UserMemoryResponse {
        address: u32,
        data: Vec<u8>,
    },
    /// `A_UserMemory_Write-PDU` (Figure 81); `number` is `data.len()`,
    /// 1 to 15.
    UserMemoryWrite {
        address: u32,
        data: Vec<u8>,
    },
    /// `A_Restart-PDU` (spec §8): `restart_type` 0 is the unconfirmed
    /// Basic Restart, 1 the confirmed Master Reset. `data` carries what
    /// the type dictates — Erase Code and Channel Number on a Master
    /// Reset request, Error Code and Process Time on its response — and is
    /// not policed here: which octets belong to which type is a procedure
    /// question (spec §8.1), not a framing one, and the frame layer
    /// refusing to guess is what keeps a Master Reset from being
    /// assembled by accident.
    Restart {
        response: bool,
        restart_type: u8,
        data: Vec<u8>,
    },
    /// `A_Authorize_Request-PDU` (AL §3.5.7 Figure 86, cited in spec
    /// §11.1): one *"must be 0"* octet, then four key octets. That octet
    /// is not a field because there is nothing to decide about it.
    AuthorizeRequest {
        key: [u8; 4],
    },
    /// `A_Authorize_Response-PDU` (Figure 87): one level octet, where a
    /// *lower* number is more powerful (spec §10.1).
    AuthorizeResponse {
        level: u8,
    },
    /// `A_PropertyValue_Read-PDU` (spec §5.1, §7.3): object index,
    /// property id, then `nr_of_elem` (4 bits) and `start_index`
    /// (12 bits) packed into two octets.
    PropertyValueRead {
        object_index: u8,
        property_id: u8,
        nr_of_elem: u8,
        start_index: u16,
    },
    /// `A_PropertyValue_Response-PDU`. A `nr_of_elem` of zero with no
    /// data is how a device refuses a property read (spec §10.6), so it
    /// is representable rather than rejected.
    PropertyValueResponse {
        object_index: u8,
        property_id: u8,
        nr_of_elem: u8,
        start_index: u16,
        data: Vec<u8>,
    },
    /// `A_PropertyValue_Write-PDU` — the service every load-state event
    /// of spec §7.3 travels on, ten octets of payload at a time.
    PropertyValueWrite {
        object_index: u8,
        property_id: u8,
        nr_of_elem: u8,
        start_index: u16,
        data: Vec<u8>,
    },
    /// The payload of a control PDU (`Tpci::Connect`/`Disconnect`/
    /// `Ack`/`Nak`): the brief's term for it, not a general-purpose
    /// "empty" state. These TPDUs carry no application layer at all
    /// (Transport Layer v01.02.03 AS §2) — the frame's `service` field
    /// still needs a value, and this is it.
    NoApplicationPdu,
    /// Any 10-bit APCI this cycle does not interpret — the two APCI-high
    /// bits (from the TPCI octet's low two bits) combined with the
    /// APCI-low octet, and every data octet that followed, so nothing is
    /// lost (CLAUDE.md: never silently discard). The frame's `Tpci` is
    /// carried separately in `LDataFrame::transport`, not folded in here.
    Other {
        apci: u16,
        data: Vec<u8>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LDataFrame {
    pub kind: LDataMessageKind,
    pub source: IndividualAddress,
    pub destination: Destination,
    pub transport: Tpci,
    pub service: ApplicationService,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CemiError {
    TooShort {
        needed: usize,
        got: usize,
    },
    UnsupportedMessageCode(u8),
    /// A `NumberedData`/`Ack`/`Nak` `seq` above 15 — the Transport Control
    /// Field's SeqNo is a 4-bit field (Transport Layer v01.02.03 AS §2,
    /// Figure 3, cited on `Tpci`). Rejected on encode rather than masked
    /// with `& 0x0F`, which would silently turn it into a different, valid
    /// sequence number a live connection would answer to the wrong state
    /// for (the same bug class Task 1 fixed for area/line, `cb673e6`).
    InvalidSequenceNumber(u8),
    /// A device-descriptor `descriptor_type` above `0x3F` — the field is
    /// six bits (Application Layer v02.01.01 AS §3.4.2.1, Figures 36/38,
    /// octet 7 bits 5-0). Those six bits share their octet with the two
    /// bits that select `Read` vs. `Response`, so an out-of-range value
    /// would silently flip which service the frame decodes as. Rejected on
    /// encode rather than masked, same reasoning as `InvalidSequenceNumber`.
    InvalidDescriptorType(u8),
    /// An `ApplicationService::Other { apci, .. }` above `0x3FF` — the APCI
    /// is a 10-bit field (Application Layer v02.01.01 AS §2.2 Table 1's
    /// APCI column). Rejected on encode rather than masked.
    InvalidApci(u16),
    /// The encoded NPDU is longer than the `L` octet can name — `L` is one
    /// octet holding `npdu.len() - 1` (EMI_IMI v01.04.02 AS §4.1.5.3.2), so
    /// 256 octets still fits and 257 is the first length that does not. `got` is the
    /// measured NPDU length. `ApplicationService::GroupValueWrite`/
    /// `GroupValueResponse` with a long DPT-24/28 string are the paths
    /// that can reach this; rejected here rather than truncated, which
    /// would silently emit a frame with a wrong-but-valid-looking `L`.
    NpduTooLong {
        got: usize,
    },
    /// A control PDU (`Tpci::Connect`/`Disconnect`/`Ack`/`Nak`, or an
    /// `Unknown` octet with bit 7 set) carried octets after its TPCI octet.
    /// These TPDUs have no field the Standard defines for them to live in
    /// (Transport Layer v01.02.03 AS §2) — inventing one would misrepresent
    /// the frame, so it is rejected instead of the octets being silently
    /// dropped (Global Constraint 2). `extra_octets` is how many followed
    /// the TPCI octet.
    UnexpectedControlPduData {
        tpci: u8,
        extra_octets: usize,
    },
    /// `LDataFrame::transport`/`LDataFrame::service` disagree, at encode
    /// time, about whether this is a control PDU: a control `Tpci` (see
    /// `is_control_pdu`) requires `ApplicationService::NoApplicationPdu`
    /// and nothing else, and `NoApplicationPdu` requires a control `Tpci`.
    /// Naming both variant names rather than emitting a frame that would
    /// not survive its own round trip.
    /// An `A_Memory_*` or `A_UserMemory_*` PDU whose octet count does not
    /// fit its `number` field — six bits (1-63) for `A_Memory_*`, four
    /// (1-15) for `A_UserMemory_*` (Application Layer v02.01.01 AS §3.4.4
    /// and §3.5.6.2). `max` says which of the two limits was applied.
    /// Rejected on encode: truncating the data to fit would write a
    /// silently shorter region than the caller asked for, at the right
    /// address, which is the worst available outcome.
    /// Unreachable in a correct build: a management service (spec §6.6)
    /// that `encode_management` did not claim, reached by the borrowed
    /// encode path that cannot build an owned payload. It names the
    /// service rather than panicking, so a future variant whose encoder
    /// arm was forgotten costs one frame and not the process — the same
    /// choice the `NoApplicationPdu` arm of `encode_l_data` already makes.
    ManagementServiceNotEncoded(&'static str),
    MemoryOctetCountOutOfRange {
        number: usize,
        max: u8,
    },
    /// An `A_UserMemory_*` address above the 20 bits the service has for
    /// it (4-bit extension + 16-bit address). Masking would write to a
    /// real but different address.
    UserMemoryAddressOutOfRange(u32),
    /// An `A_PropertyValue_*` PDU whose `nr_of_elem` exceeds four bits or
    /// whose `start_index` exceeds twelve — the two share two octets, so
    /// an overflow in either corrupts the other.
    PropertyRequestOutOfRange {
        nr_of_elem: u8,
        start_index: u16,
    },
    /// An `A_Restart` `restart_type` above 1: the field is one bit (spec
    /// §8), and 0/1 are Basic Restart and Master Reset. A value of 2 is
    /// not a third kind of reset, it is a bug.
    InvalidRestartType(u8),
    MismatchedTransport {
        transport: &'static str,
        service: &'static str,
    },
}

impl std::fmt::Display for CemiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CemiError::TooShort { needed, got } => {
                write!(
                    f,
                    "cEMI frame too short: needed at least {needed} octets, got {got}"
                )
            }
            CemiError::UnsupportedMessageCode(code) => {
                write!(f, "unsupported cEMI message code {code:#04x}")
            }
            CemiError::InvalidSequenceNumber(seq) => {
                write!(f, "TPCI sequence number {seq} does not fit 4 bits (0-15)")
            }
            CemiError::InvalidDescriptorType(descriptor_type) => {
                write!(
                    f,
                    "device descriptor type {descriptor_type:#04x} does not fit 6 bits (0-0x3F)"
                )
            }
            CemiError::InvalidApci(apci) => {
                write!(f, "APCI {apci:#06x} does not fit 10 bits (0-0x3FF)")
            }
            CemiError::NpduTooLong { got } => {
                write!(
                    f,
                    "NPDU is {got} octets, but the cEMI length octet can only name up to 255"
                )
            }
            CemiError::UnexpectedControlPduData { tpci, extra_octets } => {
                write!(
                    f,
                    "control PDU (TPCI {tpci:#04x}) carried {extra_octets} unexpected \
                     trailing octet(s)"
                )
            }
            CemiError::ManagementServiceNotEncoded(service) => {
                write!(f, "management service {service} has no encoder arm")
            }
            CemiError::MemoryOctetCountOutOfRange { number, max } => {
                write!(
                    f,
                    "memory service octet count {number} does not fit its number field (1-{max})"
                )
            }
            CemiError::UserMemoryAddressOutOfRange(address) => {
                write!(
                    f,
                    "user-memory address {address:#07x} does not fit 20 bits (0-0xFFFFF)"
                )
            }
            CemiError::PropertyRequestOutOfRange {
                nr_of_elem,
                start_index,
            } => {
                write!(
                    f,
                    "property request nr_of_elem {nr_of_elem} (max 15) / start_index \
                     {start_index} (max 0xFFF) does not fit its two octets"
                )
            }
            CemiError::InvalidRestartType(restart_type) => {
                write!(
                    f,
                    "restart type {restart_type} is not 0 (Basic Restart) or 1 (Master Reset)"
                )
            }
            CemiError::MismatchedTransport { transport, service } => {
                write!(
                    f,
                    "transport {transport} cannot carry service {service} (a control TPCI \
                     requires NoApplicationPdu, and NoApplicationPdu requires a control TPCI)"
                )
            }
        }
    }
}

impl std::error::Error for CemiError {}

/// The cEMI Additional Information type 'RF medium information' (`02h`).
const ADD_INFO_RF_MEDIUM: u8 = 0x02;

/// The 'RF medium information' of an RF frame, EMI_IMI v01.04.02 AS
/// §4.1.4.3.2, p. 61: Type ID `02h`, Len `08h`, then RF-Info (1), SN/DoA
/// (6), LFN (1). *"'RF medium information' is mandatory for RF frames."*
///
/// `[D]` The same PDF's own frame examples (§4.1.4.3.10, p. 74) draw `Len
/// = 7`, without the LFN octet — the pre-AN168 layout. Both are read; a
/// seven-octet one has `lfn: None`. Written, it is always the eight-octet
/// form the clause defines.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RfMediumInfo {
    /// RF-Info (formerly RF-Ctrl): Route Last (b7), signal strength
    /// (b5-4, b3-2), battery state (b1), unidirectional (b0). Kept whole.
    pub info: u8,
    /// A KNX Serial Number when the frame is a system broadcast (Ctrl1 SB
    /// `0`), otherwise the RF Domain Address (§4.1.4.3.9). All zero on a
    /// request means *"insert your own"* (the RF-'SN' table, p. 62).
    pub serial_or_domain: [u8; 6],
    /// The Data Link Layer frame number, `0..=7`, or `255` for *"insert
    /// your own"*. `None` for the seven-octet form.
    pub lfn: Option<u8>,
}

/// The RF-LFN value that asks the cEMI Server for its own frame number
/// (EMI_IMI §4.1.4.3.2, p. 63: *"if LFN = 255 (void): The cEMI Server shall
/// insert its own local Data Link Layer frame number"*).
pub const RF_LFN_VOID: u8 = 0xFF;

/// Walks the Additional Information TLVs (EMI_IMI §4.1.4.3.1: *"Each
/// Additional Information Type shall be accompanied by a length
/// information"*) and returns the RF medium information, if any. A TLV
/// list that runs past its own end yields `None`: before K16 this decoder
/// skipped additional information unread, and it still does for every
/// type but this one.
fn rf_medium_info(add_info: &[u8]) -> Option<RfMediumInfo> {
    let mut rest = add_info;
    while let [type_id, len, tail @ ..] = rest {
        let len = *len as usize;
        if tail.len() < len {
            return None;
        }
        let (value, next) = tail.split_at(len);
        if *type_id == ADD_INFO_RF_MEDIUM && (len == 7 || len == 8) {
            let mut serial_or_domain = [0; 6];
            serial_or_domain.copy_from_slice(&value[1..7]);
            return Some(RfMediumInfo {
                info: value[0],
                serial_or_domain,
                lfn: value.get(7).copied(),
            });
        }
        rest = next;
    }
    None
}

/// The RF medium information of a cEMI `L_Data` message, or `None` for a
/// frame of another medium (or a message too short to have any).
pub fn decode_rf_medium_info(buf: &[u8]) -> Option<RfMediumInfo> {
    let add_info_len = *buf.get(1)? as usize;
    rf_medium_info(buf.get(2..2 + add_info_len)?)
}

/// [`encode_l_data`] for KNX RF: the same frame with the mandatory 'RF
/// medium information' in front and `L` void (`00h`), EMI_IMI §4.1.5.4.1:
/// *"The corresponding 'L' field in the cEMI L_Data structure shall be void
/// and the cEMI Client shall insert the value 00h."*
pub fn encode_l_data_rf(frame: &LDataFrame, rf: &RfMediumInfo) -> Result<Vec<u8>, CemiError> {
    let plain = encode_l_data(frame)?;
    let mut buf = Vec::with_capacity(plain.len() + 10);
    buf.push(plain[0]);
    buf.push(10); // AddIL: Type ID + Len + 8 octets
    buf.push(ADD_INFO_RF_MEDIUM);
    buf.push(8);
    buf.push(rf.info);
    buf.extend_from_slice(&rf.serial_or_domain);
    buf.push(rf.lfn.unwrap_or(RF_LFN_VOID));
    // `plain` has no additional information of its own (AddIL `00h`).
    buf.extend_from_slice(&plain[2..8]);
    buf.push(0x00); // L void on RF
    buf.extend_from_slice(&plain[9..]);
    Ok(buf)
}

/// Decodes an `L_Data.req`/`.con`/`.ind` cEMI frame (EMI_IMI v01.04.02 AS
/// §4.1.5.3.2, generic layout shared by all three services).
pub fn decode_l_data(buf: &[u8]) -> Result<LDataFrame, CemiError> {
    if buf.len() < 2 {
        return Err(CemiError::TooShort {
            needed: 2,
            got: buf.len(),
        });
    }
    let message_code = buf[0];
    let kind = match message_code {
        L_DATA_REQ => LDataMessageKind::Request,
        L_DATA_IND => LDataMessageKind::Indication,
        L_DATA_CON => LDataMessageKind::Confirmation { error: false }, // filled in below
        other => return Err(CemiError::UnsupportedMessageCode(other)),
    };
    let add_info_len = buf[1] as usize;
    let fixed_part_start = 2 + add_info_len;
    // Ctrl1, Ctrl2, 2*source, 2*destination, L = 7 octets minimum before
    // the TPCI/APCI octets even start.
    if buf.len() < fixed_part_start + 7 {
        return Err(CemiError::TooShort {
            needed: fixed_part_start + 7,
            got: buf.len(),
        });
    }
    let ctrl1 = buf[fixed_part_start];
    let ctrl2 = buf[fixed_part_start + 1];
    let kind = match kind {
        LDataMessageKind::Confirmation { .. } => LDataMessageKind::Confirmation {
            error: ctrl1 & 0x01 != 0,
        },
        other => other,
    };
    let source = IndividualAddress::from_raw(u16::from_be_bytes([
        buf[fixed_part_start + 2],
        buf[fixed_part_start + 3],
    ]));
    let dest_raw = u16::from_be_bytes([buf[fixed_part_start + 4], buf[fixed_part_start + 5]]);
    // Ctrl2 bit 7: Address Type — 0 individual, 1 group (EMI_IMI v01.04.02
    // AS §4.1.5.3.2).
    // A group frame to `0000h` with SB clear is the system broadcast
    // (see `Destination::SystemBroadcast`); SB means nothing on any other
    // destination and is not judged there.
    let destination = if ctrl2 & 0x80 != 0 {
        if dest_raw == 0 && ctrl1 & CTRL1_SB_BROADCAST == 0 {
            Destination::SystemBroadcast
        } else {
            Destination::Group(GroupAddress::from_raw(dest_raw))
        }
    } else {
        Destination::Individual(IndividualAddress::from_raw(dest_raw))
    };
    let tpci_apci_start = fixed_part_start + 7;
    // EMI_IMI §4.1.5.4.1/§4.1.5.4.3: an RF frame's `L` is void (`00h`),
    // because the RF frame carries no NPDU length; the NPDU runs to the
    // end of the message. An RF frame is one with the mandatory 'RF
    // medium information' (§4.1.4.3.2).
    let length = if rf_medium_info(&buf[2..fixed_part_start]).is_some() {
        buf.len().saturating_sub(tpci_apci_start + 1)
    } else {
        buf[fixed_part_start + 6] as usize
    };
    // Every TPDU has at least a TPCI octet; a control PDU
    // (`Connect`/`Disconnect`/`Ack`/`Nak`) has nothing else, so that much
    // is the minimum this function can demand up front.
    if buf.len() < tpci_apci_start + 1 {
        return Err(CemiError::TooShort {
            needed: tpci_apci_start + 1,
            got: buf.len(),
        });
    }
    let tpci_octet = buf[tpci_apci_start];
    let transport = decode_tpci(tpci_octet);
    let service = if is_control_pdu(transport) {
        // `Tpci::Connect`/`Disconnect`/`Ack`/`Nak` (and any `Unknown`
        // octet with bit 7 set) carry no application layer at all
        // (Transport Layer v01.02.03 AS §2) — decoding them must not
        // demand the APCI octet the data-PDU path below needs. A
        // well-formed one has `L = 0` (its TPDU is the TPCI octet alone);
        // anything else means octets follow that this TPDU shape has no
        // field for, so they are reported rather than silently dropped
        // (Global Constraint 2) — see `CemiError::UnexpectedControlPduData`.
        if length != 0 {
            let extra_start = tpci_apci_start + 1;
            if buf.len() < extra_start + length {
                return Err(CemiError::TooShort {
                    needed: extra_start + length,
                    got: buf.len(),
                });
            }
            return Err(CemiError::UnexpectedControlPduData {
                tpci: tpci_octet,
                extra_octets: length,
            });
        }
        ApplicationService::NoApplicationPdu
    } else {
        if buf.len() < tpci_apci_start + 2 {
            return Err(CemiError::TooShort {
                needed: tpci_apci_start + 2,
                got: buf.len(),
            });
        }
        let apci_lo_and_data = buf[tpci_apci_start + 1];
        // The three `A_GroupValue_*` services, and both
        // `A_DeviceDescriptor_*` services, share a 4-bit APCI selector
        // (Application Layer v02.01.01 AS §2.2 Table 1 / §3.4.2.1): its
        // top 2 bits are TPCI-octet bits 1-0, its bottom 2 bits are
        // APCI-octet bits 7-6. The APCI-octet's remaining 6 bits (bits
        // 5-0) carry inline data (`GroupValue::Short`, `length <= 1`) or,
        // for device descriptor services, `descriptor_type`.
        let short_apci = ((tpci_octet & 0x03) << 2) | (apci_lo_and_data >> 6);
        let inline6 = apci_lo_and_data & 0x3F;
        let extra_len = length.saturating_sub(1);
        let extra_start = tpci_apci_start + 2;
        if buf.len() < extra_start + extra_len {
            return Err(CemiError::TooShort {
                needed: extra_start + extra_len,
                got: buf.len(),
            });
        }
        let extra = &buf[extra_start..extra_start + extra_len];
        match short_apci {
            0b0000 => ApplicationService::GroupValueRead,
            0b0001 => ApplicationService::GroupValueResponse(group_value(length, inline6, extra)),
            0b0010 => ApplicationService::GroupValueWrite(group_value(length, inline6, extra)),
            // `A_DeviceDescriptor_Read-PDU` never carries data octets
            // (Application Layer v02.01.01 AS §3.4.2.1); a frame that
            // matches its APCI but carries trailing octets anyway does not
            // fit that PDU shape, so it falls to `Other` below, which
            // preserves them instead of quietly dropping them.
            0b1100 if extra.is_empty() => ApplicationService::DeviceDescriptorRead {
                descriptor_type: inline6,
            },
            0b1101 => ApplicationService::DeviceDescriptorResponse {
                descriptor_type: inline6,
                data: extra.to_vec(),
            },
            _ => {
                let apci = (((tpci_octet & 0x03) as u16) << 8) | apci_lo_and_data as u16;
                // The §6.6 management services, which all live in APCIs
                // this `match` does not select on. `None` means the octets
                // did not fit the PDU shape that APCI names, and the frame
                // falls through to `Other` with everything intact — the
                // same treatment `A_DeviceDescriptor_Read` with unexpected
                // trailing octets already gets above.
                decode_management(apci, extra).unwrap_or(ApplicationService::Other {
                    apci,
                    data: extra.to_vec(),
                })
            }
        }
    };
    Ok(LDataFrame {
        kind,
        source,
        destination,
        transport,
        service,
    })
}

/// True for every TPCI whose TPDU is the TPCI octet alone — no APCI, no
/// application layer (Transport Layer v01.02.03 AS §2): the four named
/// control PDUs, plus an `Unknown` octet with bit 7 (Data/Control) set,
/// since every named control PDU lives in that half of the octet space and
/// every named data PDU lives in the other half (see `Tpci::Unknown`'s doc
/// comment). Takes `Tpci` by value — it is `Copy`, a two-field enum at
/// most, so a reference buys nothing here.
fn is_control_pdu(tpci: Tpci) -> bool {
    match tpci {
        Tpci::Connect | Tpci::Disconnect | Tpci::Ack { .. } | Tpci::Nak { .. } => true,
        Tpci::UnnumberedData | Tpci::NumberedData { .. } => false,
        Tpci::Unknown(octet) => octet & 0x80 != 0,
    }
}

/// Decodes octet 6, the Transport Control Field — bit table and citation
/// on `Tpci`'s doc comment. Infallible: every octet value maps to a
/// `Tpci`, `Unknown` catching whatever the named variants do not
/// (`T_Data_Tag_Group`, NOTE 1's reserved `0xBF`, or anything else) —
/// reported via a distinct, inspectable value rather than an error the
/// only two current callers (`client.rs`, both `if let Ok(...)`) would
/// have swallowed, silently discarding the whole frame (Global
/// Constraint 2).
fn decode_tpci(octet: u8) -> Tpci {
    match octet & 0xC0 {
        0x00 => {
            // Data/Control=0, Numbered=0: `T_Data_Broadcast`/`_Group`/
            // `_Individual` all share this pattern with bits 5-2 zero.
            // `T_Data_Tag_Group` (`0x04`) sets bit 2 and is not modeled —
            // `Unknown`, not coerced into `UnnumberedData`. Masked to bits
            // 7-2 (`& 0xFC`): bits 1-0 are the short-APCI's top two bits,
            // not transport's (see `Tpci::Unknown`'s doc comment) — storing
            // them here would let a later `A_GroupValue_*`/device-descriptor
            // pairing on the same frame corrupt this field on re-encode, or
            // corrupt `service` on a hand-built frame's re-decode.
            if octet & 0x3C != 0 {
                Tpci::Unknown(octet & 0xFC)
            } else {
                Tpci::UnnumberedData
            }
        }
        0x40 => Tpci::NumberedData {
            seq: (octet >> 2) & 0x0F,
        },
        0x80 => match octet {
            0x80 => Tpci::Connect,
            0x81 => Tpci::Disconnect,
            _ => Tpci::Unknown(octet),
        },
        _ => {
            // 0xC0: Data/Control=1, Numbered=1 — `T_ACK`/`T_NAK`,
            // distinguished by bits 1-0 (`10`/`11`); bits 1-0 = `00`/`01`
            // here are unrecognized (NOTE 1's reserved `0xBF` falls in the
            // 0x80 branch above instead, since its bit 6 is 0).
            let seq = (octet >> 2) & 0x0F;
            match octet & 0x03 {
                0b10 => Tpci::Ack { seq },
                0b11 => Tpci::Nak { seq },
                _ => Tpci::Unknown(octet),
            }
        }
    }
}

fn group_value(length: usize, inline6: u8, extra: &[u8]) -> GroupValue {
    if length <= 1 {
        GroupValue::Short(inline6)
    } else {
        GroupValue::Bytes(extra.to_vec())
    }
}

/// Encodes an `L_Data.req`/`.con`/`.ind` cEMI frame — the inverse of
/// `decode_l_data`, same fixed layout (EMI_IMI v01.04.02 AS §4.1.5.3.2),
/// never emitting additional information (`add_info_len = 0`; this cycle
/// never needs any). Ctrl1 0xBC (standard frame, no repeat, domain
/// broadcast, low priority, no ack request) and Ctrl2's hop-count-6 are
/// this crate's outbound defaults — the same values already implied by
/// every hand-built fixture `decode_l_data` is tested against above. The
/// exception is a Transport Layer control request (`T_CONNECT`,
/// `T_DISCONNECT`, `T_ACK`, `T_NAK`), sent at system priority per TL §3.7,
/// §3.8 and §5.3 (see [`CTRL1_SYSTEM_ACK_REQUESTED`], [`CTRL1_SYSTEM`]).
/// Names a management service (spec §6.6) from its APCI and data octets,
/// or returns `None` if the octets do not fit that service's PDU.
///
/// Deliberately shape-based and not value-based: it checks the octet count
/// the Standard's figures give each PDU and nothing else. A device that
/// answers with a field value no procedure expects is still reported as
/// the service it claimed to be — judging values is the procedure layer's
/// job in `knx-core`, and a decoder that demotes a badly-filled frame to
/// `Other` would hide exactly the misbehaviour spec §9.2 wants named.
fn decode_management(apci: u16, extra: &[u8]) -> Option<ApplicationService> {
    let be16 = |a: u8, b: u8| u16::from_be_bytes([a, b]);
    let serial = |extra: &[u8]| -> [u8; 6] {
        let mut serial_number = [0; 6];
        serial_number.copy_from_slice(&extra[..6]);
        serial_number
    };
    // `A_UserMemory_*`: octet 8 is the address extension (bits 7-4) and
    // `number` (bits 3-0), octets 9-10 the 16-bit address (Figure 79).
    let user_memory = |extra: &[u8]| -> (u8, u32) {
        let number = extra[0] & 0x0F;
        let address = ((extra[0] >> 4) as u32) << 16 | be16(extra[1], extra[2]) as u32;
        (number, address)
    };
    match apci {
        APCI_INDIVIDUAL_ADDRESS_WRITE if extra.len() == 2 => {
            Some(ApplicationService::IndividualAddressWrite {
                address: IndividualAddress::from_raw(be16(extra[0], extra[1])),
            })
        }
        APCI_INDIVIDUAL_ADDRESS_READ if extra.is_empty() => {
            Some(ApplicationService::IndividualAddressRead)
        }
        APCI_INDIVIDUAL_ADDRESS_RESPONSE if extra.is_empty() => {
            Some(ApplicationService::IndividualAddressResponse)
        }
        // Shape as the figures draw it. A longer PDU is not this service;
        // reserved octets are not judged (see the function's docs).
        APCI_IA_SERIAL_NUMBER_READ if extra.len() == 6 => {
            Some(ApplicationService::IndividualAddressSerialNumberRead {
                serial_number: serial(extra),
            })
        }
        APCI_IA_SERIAL_NUMBER_RESPONSE if extra.len() == 10 => {
            Some(ApplicationService::IndividualAddressSerialNumberResponse {
                serial_number: serial(extra),
                domain_address: be16(extra[6], extra[7]),
            })
        }
        APCI_IA_SERIAL_NUMBER_WRITE if extra.len() == 12 => {
            Some(ApplicationService::IndividualAddressSerialNumberWrite {
                serial_number: serial(extra),
                address: IndividualAddress::from_raw(be16(extra[6], extra[7])),
            })
        }
        APCI_DOMAIN_ADDRESS_WRITE => DomainAddress::from_octets(extra)
            .map(|domain_address| ApplicationService::DomainAddressWrite { domain_address }),
        APCI_DOMAIN_ADDRESS_READ if extra.is_empty() => Some(ApplicationService::DomainAddressRead),
        APCI_DOMAIN_ADDRESS_RESPONSE => DomainAddress::from_octets(extra)
            .map(|domain_address| ApplicationService::DomainAddressResponse { domain_address }),
        APCI_DOA_SERIAL_NUMBER_READ if extra.len() == 6 => {
            Some(ApplicationService::DomainAddressSerialNumberRead {
                serial_number: serial(extra),
            })
        }
        APCI_DOA_SERIAL_NUMBER_RESPONSE if extra.len() > 6 => {
            DomainAddress::from_octets(&extra[6..]).map(|domain_address| {
                ApplicationService::DomainAddressSerialNumberResponse {
                    serial_number: serial(extra),
                    domain_address,
                }
            })
        }
        // The 4-octet KNX IP form falls to `Other`, octets intact.
        APCI_DOA_SERIAL_NUMBER_WRITE if extra.len() > 6 => DomainAddress::from_octets(&extra[6..])
            .map(
                |domain_address| ApplicationService::DomainAddressSerialNumberWrite {
                    serial_number: serial(extra),
                    domain_address,
                },
            ),
        APCI_USER_MEMORY_READ if extra.len() == 3 => {
            let (number, address) = user_memory(extra);
            Some(ApplicationService::UserMemoryRead { number, address })
        }
        APCI_USER_MEMORY_RESPONSE | APCI_USER_MEMORY_WRITE if extra.len() >= 3 => {
            let (number, address) = user_memory(extra);
            if number as usize != extra.len() - 3 {
                return None;
            }
            let data = extra[3..].to_vec();
            Some(if apci == APCI_USER_MEMORY_RESPONSE {
                ApplicationService::UserMemoryResponse { address, data }
            } else {
                ApplicationService::UserMemoryWrite { address, data }
            })
        }
        APCI_AUTHORIZE_REQUEST if extra.len() == 5 && extra[0] == 0 => {
            Some(ApplicationService::AuthorizeRequest {
                key: [extra[1], extra[2], extra[3], extra[4]],
            })
        }
        APCI_AUTHORIZE_RESPONSE if extra.len() == 1 => {
            Some(ApplicationService::AuthorizeResponse { level: extra[0] })
        }
        APCI_PROPERTY_VALUE_READ if extra.len() == 4 => {
            Some(ApplicationService::PropertyValueRead {
                object_index: extra[0],
                property_id: extra[1],
                nr_of_elem: extra[2] >> 4,
                start_index: be16(extra[2] & 0x0F, extra[3]),
            })
        }
        APCI_PROPERTY_VALUE_RESPONSE | APCI_PROPERTY_VALUE_WRITE if extra.len() >= 4 => {
            let object_index = extra[0];
            let property_id = extra[1];
            let nr_of_elem = extra[2] >> 4;
            let start_index = be16(extra[2] & 0x0F, extra[3]);
            let data = extra[4..].to_vec();
            Some(if apci == APCI_PROPERTY_VALUE_RESPONSE {
                ApplicationService::PropertyValueResponse {
                    object_index,
                    property_id,
                    nr_of_elem,
                    start_index,
                    data,
                }
            } else {
                ApplicationService::PropertyValueWrite {
                    object_index,
                    property_id,
                    nr_of_elem,
                    start_index,
                    data,
                }
            })
        }
        _ => decode_memory_or_restart(apci, extra),
    }
}

/// The two selector-plus-inline-field families: `A_Memory_*`, whose low six
/// APCI bits are `number`, and `A_Restart`, whose low six carry the
/// Response bit and the Restart Type.
fn decode_memory_or_restart(apci: u16, extra: &[u8]) -> Option<ApplicationService> {
    let number = (apci & 0x3F) as u8;
    let selector = apci & APCI_SELECTOR_MASK;
    if selector == APCI_MEMORY_READ && extra.len() == 2 {
        return Some(ApplicationService::MemoryRead {
            number,
            address: u16::from_be_bytes([extra[0], extra[1]]),
        });
    }
    if (selector == APCI_MEMORY_RESPONSE || selector == APCI_MEMORY_WRITE) && extra.len() >= 2 {
        if number as usize != extra.len() - 2 {
            return None;
        }
        let address = u16::from_be_bytes([extra[0], extra[1]]);
        let data = extra[2..].to_vec();
        return Some(if selector == APCI_MEMORY_RESPONSE {
            ApplicationService::MemoryResponse { address, data }
        } else {
            ApplicationService::MemoryWrite { address, data }
        });
    }
    if selector == APCI_RESTART && number & RESTART_RESERVED_BITS == 0 {
        return Some(ApplicationService::Restart {
            response: number & RESTART_RESPONSE_BIT != 0,
            restart_type: number & 0x01,
            data: extra.to_vec(),
        });
    }
    None
}

/// Ctrl1 of an outbound `T_CONNECT`/`T_DISCONNECT` request: standard frame,
/// R and SB as in 0xBC, priority `00b` (system), ack requested (1011_0010).
/// TL v01.02.03 AS §3.7/§3.8: "the priority shall be set to 'system'; the
/// ack_request shall be set to true". Ctrl1 layout EMI_IMI v01.04.02 AS
/// §4.1.5.3.2; priority codes Data Link Layer General v01.03.02 AS §2.2.3.
const CTRL1_SYSTEM_ACK_REQUESTED: u8 = 0xB2;
/// Ctrl1 of an outbound `T_ACK`/`T_NAK`: as above, but TL §5.3 A2-A4 name
/// only "priority = SYSTEM", so ack_request keeps the crate default (clear):
/// 1011_0000.
const CTRL1_SYSTEM: u8 = 0xB0;

pub fn encode_l_data(frame: &LDataFrame) -> Result<Vec<u8>, CemiError> {
    let message_code = match frame.kind {
        LDataMessageKind::Request => L_DATA_REQ,
        LDataMessageKind::Indication => L_DATA_IND,
        LDataMessageKind::Confirmation { .. } => L_DATA_CON,
    };
    let ctrl1 = match (frame.kind, frame.transport) {
        (LDataMessageKind::Confirmation { error: true }, _) => 0xBD,
        (LDataMessageKind::Request, Tpci::Connect | Tpci::Disconnect) => CTRL1_SYSTEM_ACK_REQUESTED,
        (LDataMessageKind::Request, Tpci::Ack { .. } | Tpci::Nak { .. }) => CTRL1_SYSTEM,
        _ => 0xBC,
    };
    let (address_type_bit, dest_raw) = match frame.destination {
        Destination::Group(addr) => (0x80, addr.raw()),
        Destination::Individual(addr) => (0x00, addr.raw()),
        Destination::SystemBroadcast => (0x80, 0x0000),
    };
    let ctrl1 = if frame.destination == Destination::SystemBroadcast {
        ctrl1 & !CTRL1_SB_BROADCAST
    } else {
        ctrl1
    };
    let ctrl2 = address_type_bit | 0x60; // hop count 6, standard EFF (0000)
    let source_raw = frame.source.raw();

    // The TPCI octet's own bits (Transport Layer v01.02.03 AS §2, Figure
    // 3, cited on `Tpci`). For `UnnumberedData`/`NumberedData` its low two
    // bits are 0 here — the data-PDU path below ORs the APCI-high bits in;
    // control PDUs own the whole octet outright.
    let tpci_octet = encode_tpci(&frame.transport)?;

    // `transport`/`service` cross-field invariant (Transport Layer
    // v01.02.03 AS §2): a control `Tpci` carries no application layer at
    // all, so it pairs only with `NoApplicationPdu`, and vice versa.
    // Enforced here rather than left implicit — an unenforced pairing lets
    // a caller build a frame that does not survive its own round trip
    // (e.g. `Connect` + `GroupValueWrite` would emit a stray APCI octet a
    // `T_Connect`-shaped TPDU has no field for). This is a checked
    // encode-time invariant, not a type-level restructure that makes the
    // bad pairing unrepresentable — a deliberately smaller fix.
    let transport_is_control = is_control_pdu(frame.transport);
    let service_is_no_application_pdu =
        matches!(frame.service, ApplicationService::NoApplicationPdu);
    if transport_is_control != service_is_no_application_pdu {
        return Err(CemiError::MismatchedTransport {
            transport: tpci_variant_name(&frame.transport),
            service: application_service_variant_name(&frame.service),
        });
    }

    if service_is_no_application_pdu {
        // `Tpci::Connect`/`Disconnect`/`Ack`/`Nak` (or an `Unknown`
        // control-shaped octet): the TPDU is the TPCI octet alone — no
        // APCI, no data (Transport Layer v01.02.03 AS §2).
        return finish_l_data(
            message_code,
            ctrl1,
            ctrl2,
            source_raw,
            dest_raw,
            &[tpci_octet],
        );
    }

    // The §6.6 management services encode their whole 10-bit APCI plus an
    // owned payload, so they take this path rather than the borrowed
    // `(short_apci, inline6, extra)` one below.
    if let Some((apci, payload)) = encode_management(&frame.service)? {
        let tpci_octet = tpci_octet | (((apci >> 8) as u8) & 0x03);
        let npdu: Vec<u8> = [tpci_octet, apci as u8]
            .into_iter()
            .chain(payload)
            .collect();
        return finish_l_data(message_code, ctrl1, ctrl2, source_raw, dest_raw, &npdu);
    }

    let (short_apci, inline6, extra): (u8, u8, &[u8]) = match &frame.service {
        ApplicationService::GroupValueRead => (0b0000, 0, &[]),
        ApplicationService::GroupValueResponse(v) => {
            let (inline6, extra) = encode_group_value(v);
            (0b0001, inline6, extra)
        }
        ApplicationService::GroupValueWrite(v) => {
            let (inline6, extra) = encode_group_value(v);
            (0b0010, inline6, extra)
        }
        ApplicationService::DeviceDescriptorRead { descriptor_type } => {
            if *descriptor_type > 0x3F {
                return Err(CemiError::InvalidDescriptorType(*descriptor_type));
            }
            (0b1100, *descriptor_type, &[])
        }
        ApplicationService::DeviceDescriptorResponse {
            descriptor_type,
            data,
        } => {
            if *descriptor_type > 0x3F {
                return Err(CemiError::InvalidDescriptorType(*descriptor_type));
            }
            (0b1101, *descriptor_type, data)
        }
        ApplicationService::Other { apci, data } => {
            if *apci > 0x3FF {
                return Err(CemiError::InvalidApci(*apci));
            }
            // Round-trips exactly what `decode_l_data` reconstructs `apci`
            // from: bits 9-8 (the TPCI octet's low two bits) OR into
            // `tpci_octet`, bits 7-0 are the whole APCI-low octet.
            let tpci_octet = tpci_octet | (((apci >> 8) as u8) & 0x03);
            let apci_lo = *apci as u8;
            let npdu: Vec<u8> = [tpci_octet, apci_lo]
                .into_iter()
                .chain(data.iter().copied())
                .collect();
            return finish_l_data(message_code, ctrl1, ctrl2, source_raw, dest_raw, &npdu);
        }
        // Genuinely unreachable: `service_is_no_application_pdu` was
        // computed from this exact `matches!` above and, had it been
        // `true`, already returned. The arm still has to exist because
        // `ApplicationService` is matched exhaustively here — there is no
        // way to remove it without the type-level restructure this fix
        // deliberately does not do — but it returns an error rather than
        // `unreachable!()`: if this ever became reachable (a future edit
        // disturbing the invariant checked above), a caller on a live bus
        // gets a `Result::Err` back instead of the whole process panicking
        // mid-scan.
        ApplicationService::NoApplicationPdu => {
            return Err(CemiError::MismatchedTransport {
                transport: tpci_variant_name(&frame.transport),
                service: application_service_variant_name(&frame.service),
            });
        }
        // Every service above is encoded by `encode_management`, which
        // returned before this `match` was reached. The arm exists because
        // the `match` is total over `ApplicationService` — and it stays
        // total on purpose, so that adding a service is a compile error
        // here rather than a frame that quietly encodes as something else.
        service @ (ApplicationService::IndividualAddressWrite { .. }
        | ApplicationService::IndividualAddressRead
        | ApplicationService::IndividualAddressResponse
        | ApplicationService::IndividualAddressSerialNumberRead { .. }
        | ApplicationService::IndividualAddressSerialNumberResponse { .. }
        | ApplicationService::IndividualAddressSerialNumberWrite { .. }
        | ApplicationService::DomainAddressWrite { .. }
        | ApplicationService::DomainAddressRead
        | ApplicationService::DomainAddressResponse { .. }
        | ApplicationService::DomainAddressSerialNumberRead { .. }
        | ApplicationService::DomainAddressSerialNumberResponse { .. }
        | ApplicationService::DomainAddressSerialNumberWrite { .. }
        | ApplicationService::MemoryRead { .. }
        | ApplicationService::MemoryResponse { .. }
        | ApplicationService::MemoryWrite { .. }
        | ApplicationService::UserMemoryRead { .. }
        | ApplicationService::UserMemoryResponse { .. }
        | ApplicationService::UserMemoryWrite { .. }
        | ApplicationService::Restart { .. }
        | ApplicationService::AuthorizeRequest { .. }
        | ApplicationService::AuthorizeResponse { .. }
        | ApplicationService::PropertyValueRead { .. }
        | ApplicationService::PropertyValueResponse { .. }
        | ApplicationService::PropertyValueWrite { .. }) => {
            return Err(CemiError::ManagementServiceNotEncoded(
                application_service_variant_name(service),
            ));
        }
    };
    let tpci_octet = tpci_octet | ((short_apci >> 2) & 0x03);
    let apci_lo = ((short_apci & 0x03) << 6) | inline6;
    let npdu: Vec<u8> = [tpci_octet, apci_lo]
        .into_iter()
        .chain(extra.iter().copied())
        .collect();
    finish_l_data(message_code, ctrl1, ctrl2, source_raw, dest_raw, &npdu)
}

/// Builds the APCI and payload octets of a management service (spec §6.6),
/// or `None` if this is not one. Every field that shares an octet with
/// another is range-checked here, because a silent mask would change which
/// address gets written or which service the frame decodes as.
fn encode_management(service: &ApplicationService) -> Result<Option<(u16, Vec<u8>)>, CemiError> {
    // `number` is never stored next to the data it counts; it is derived
    // from the data's length here, so a mismatch is unrepresentable.
    fn counted(data: &[u8], max: u8, allow_empty: bool) -> Result<u8, CemiError> {
        let n = data.len();
        if n > max as usize || (n == 0 && !allow_empty) {
            return Err(CemiError::MemoryOctetCountOutOfRange { number: n, max });
        }
        Ok(n as u8)
    }
    fn user_memory_head(address: u32, number: u8) -> Result<[u8; 3], CemiError> {
        if address >= USER_MEMORY_ADDRESS_LIMIT {
            return Err(CemiError::UserMemoryAddressOutOfRange(address));
        }
        let extension = ((address >> 16) as u8) & 0x0F;
        let [_, _, high, low] = address.to_be_bytes();
        Ok([(extension << 4) | number, high, low])
    }
    fn property_head(
        object_index: u8,
        property_id: u8,
        nr_of_elem: u8,
        start_index: u16,
    ) -> Result<[u8; 4], CemiError> {
        if nr_of_elem > PROPERTY_MAX_NR_OF_ELEM || start_index > PROPERTY_MAX_START_INDEX {
            return Err(CemiError::PropertyRequestOutOfRange {
                nr_of_elem,
                start_index,
            });
        }
        let [high, low] = start_index.to_be_bytes();
        Ok([
            object_index,
            property_id,
            (nr_of_elem << 4) | (high & 0x0F),
            low,
        ])
    }

    let encoded = match service {
        ApplicationService::IndividualAddressWrite { address } => (
            APCI_INDIVIDUAL_ADDRESS_WRITE,
            address.raw().to_be_bytes().to_vec(),
        ),
        ApplicationService::IndividualAddressRead => (APCI_INDIVIDUAL_ADDRESS_READ, Vec::new()),
        ApplicationService::IndividualAddressResponse => {
            (APCI_INDIVIDUAL_ADDRESS_RESPONSE, Vec::new())
        }
        ApplicationService::IndividualAddressSerialNumberRead { serial_number } => {
            (APCI_IA_SERIAL_NUMBER_READ, serial_number.to_vec())
        }
        ApplicationService::IndividualAddressSerialNumberResponse {
            serial_number,
            domain_address,
        } => {
            let mut extra = serial_number.to_vec();
            extra.extend_from_slice(&domain_address.to_be_bytes());
            extra.extend_from_slice(&[0, 0]);
            (APCI_IA_SERIAL_NUMBER_RESPONSE, extra)
        }
        ApplicationService::IndividualAddressSerialNumberWrite {
            serial_number,
            address,
        } => {
            let mut extra = serial_number.to_vec();
            extra.extend_from_slice(&address.raw().to_be_bytes());
            extra.extend_from_slice(&[0, 0, 0, 0]);
            (APCI_IA_SERIAL_NUMBER_WRITE, extra)
        }
        ApplicationService::DomainAddressWrite { domain_address } => {
            (APCI_DOMAIN_ADDRESS_WRITE, domain_address.octets())
        }
        ApplicationService::DomainAddressRead => (APCI_DOMAIN_ADDRESS_READ, Vec::new()),
        ApplicationService::DomainAddressResponse { domain_address } => {
            (APCI_DOMAIN_ADDRESS_RESPONSE, domain_address.octets())
        }
        ApplicationService::DomainAddressSerialNumberRead { serial_number } => {
            (APCI_DOA_SERIAL_NUMBER_READ, serial_number.to_vec())
        }
        ApplicationService::DomainAddressSerialNumberResponse {
            serial_number,
            domain_address,
        } => {
            let mut extra = serial_number.to_vec();
            extra.extend(domain_address.octets());
            (APCI_DOA_SERIAL_NUMBER_RESPONSE, extra)
        }
        ApplicationService::DomainAddressSerialNumberWrite {
            serial_number,
            domain_address,
        } => {
            let mut extra = serial_number.to_vec();
            extra.extend(domain_address.octets());
            (APCI_DOA_SERIAL_NUMBER_WRITE, extra)
        }
        ApplicationService::MemoryRead { number, address } => {
            if *number == 0 || *number > MEMORY_MAX_OCTETS {
                return Err(CemiError::MemoryOctetCountOutOfRange {
                    number: *number as usize,
                    max: MEMORY_MAX_OCTETS,
                });
            }
            (
                APCI_MEMORY_READ | *number as u16,
                address.to_be_bytes().to_vec(),
            )
        }
        ApplicationService::MemoryResponse { address, data } => {
            // A response may legitimately be empty: `number = 0` with no
            // data is how a device reports a failed read (spec §6.1).
            let number = counted(data, MEMORY_MAX_OCTETS, true)?;
            let mut payload = address.to_be_bytes().to_vec();
            payload.extend_from_slice(data);
            (APCI_MEMORY_RESPONSE | number as u16, payload)
        }
        ApplicationService::MemoryWrite { address, data } => {
            let number = counted(data, MEMORY_MAX_OCTETS, false)?;
            let mut payload = address.to_be_bytes().to_vec();
            payload.extend_from_slice(data);
            (APCI_MEMORY_WRITE | number as u16, payload)
        }
        ApplicationService::UserMemoryRead { number, address } => {
            if *number == 0 || *number > USER_MEMORY_MAX_OCTETS {
                return Err(CemiError::MemoryOctetCountOutOfRange {
                    number: *number as usize,
                    max: USER_MEMORY_MAX_OCTETS,
                });
            }
            (
                APCI_USER_MEMORY_READ,
                user_memory_head(*address, *number)?.to_vec(),
            )
        }
        ApplicationService::UserMemoryResponse { address, data } => {
            let number = counted(data, USER_MEMORY_MAX_OCTETS, true)?;
            let mut payload = user_memory_head(*address, number)?.to_vec();
            payload.extend_from_slice(data);
            (APCI_USER_MEMORY_RESPONSE, payload)
        }
        ApplicationService::UserMemoryWrite { address, data } => {
            let number = counted(data, USER_MEMORY_MAX_OCTETS, false)?;
            let mut payload = user_memory_head(*address, number)?.to_vec();
            payload.extend_from_slice(data);
            (APCI_USER_MEMORY_WRITE, payload)
        }
        ApplicationService::Restart {
            response,
            restart_type,
            data,
        } => {
            if *restart_type > 1 {
                return Err(CemiError::InvalidRestartType(*restart_type));
            }
            let inline6 = if *response { RESTART_RESPONSE_BIT } else { 0 } | *restart_type;
            (APCI_RESTART | inline6 as u16, data.clone())
        }
        ApplicationService::AuthorizeRequest { key } => {
            // The leading octet is the Standard's *"must be 0"* reserved
            // octet (AL §3.5.7 Figure 86), written here so no call site
            // has to remember it.
            let mut payload = vec![0u8];
            payload.extend_from_slice(key);
            (APCI_AUTHORIZE_REQUEST, payload)
        }
        ApplicationService::AuthorizeResponse { level } => (APCI_AUTHORIZE_RESPONSE, vec![*level]),
        ApplicationService::PropertyValueRead {
            object_index,
            property_id,
            nr_of_elem,
            start_index,
        } => (
            APCI_PROPERTY_VALUE_READ,
            property_head(*object_index, *property_id, *nr_of_elem, *start_index)?.to_vec(),
        ),
        ApplicationService::PropertyValueResponse {
            object_index,
            property_id,
            nr_of_elem,
            start_index,
            data,
        } => {
            let mut payload =
                property_head(*object_index, *property_id, *nr_of_elem, *start_index)?.to_vec();
            payload.extend_from_slice(data);
            (APCI_PROPERTY_VALUE_RESPONSE, payload)
        }
        ApplicationService::PropertyValueWrite {
            object_index,
            property_id,
            nr_of_elem,
            start_index,
            data,
        } => {
            let mut payload =
                property_head(*object_index, *property_id, *nr_of_elem, *start_index)?.to_vec();
            payload.extend_from_slice(data);
            (APCI_PROPERTY_VALUE_WRITE, payload)
        }
        _ => return Ok(None),
    };
    Ok(Some(encoded))
}

/// The `Tpci` variant name, for `CemiError::MismatchedTransport` — names
/// the offending pairing without needing `CemiError` to own a `Tpci`.
fn tpci_variant_name(tpci: &Tpci) -> &'static str {
    match tpci {
        Tpci::UnnumberedData => "UnnumberedData",
        Tpci::NumberedData { .. } => "NumberedData",
        Tpci::Connect => "Connect",
        Tpci::Disconnect => "Disconnect",
        Tpci::Ack { .. } => "Ack",
        Tpci::Nak { .. } => "Nak",
        Tpci::Unknown(_) => "Unknown",
    }
}

/// The `ApplicationService` variant name, for `CemiError::
/// MismatchedTransport` — names the offending pairing without needing
/// `CemiError` to own a non-`Copy` `ApplicationService`.
impl ApplicationService {
    /// The variant's name, for logs and monitors. Public so a renderer
    /// outside this crate can name a service without a 20-arm `match` of
    /// its own — and so adding a service here does not break one.
    pub fn variant_name(&self) -> &'static str {
        application_service_variant_name(self)
    }

    /// A one-line rendering of the service's own fields, for a monitor
    /// row. `None` for the services that have no fields worth a line.
    pub fn payload_summary(&self) -> Option<String> {
        match self {
            ApplicationService::IndividualAddressWrite { address } => Some(format!("{address}")),
            ApplicationService::IndividualAddressSerialNumberRead { serial_number } => {
                Some(format!("sn={}", format_serial_number(serial_number)))
            }
            ApplicationService::IndividualAddressSerialNumberResponse {
                serial_number,
                domain_address,
            } => Some(format!(
                "sn={} domain={domain_address:#06x}",
                format_serial_number(serial_number)
            )),
            ApplicationService::IndividualAddressSerialNumberWrite {
                serial_number,
                address,
            } => Some(format!(
                "sn={} new={address}",
                format_serial_number(serial_number)
            )),
            ApplicationService::DomainAddressWrite { domain_address }
            | ApplicationService::DomainAddressResponse { domain_address } => {
                Some(format!("domain={domain_address}"))
            }
            ApplicationService::DomainAddressSerialNumberRead { serial_number } => {
                Some(format!("sn={}", format_serial_number(serial_number)))
            }
            ApplicationService::DomainAddressSerialNumberResponse {
                serial_number,
                domain_address,
            }
            | ApplicationService::DomainAddressSerialNumberWrite {
                serial_number,
                domain_address,
            } => Some(format!(
                "sn={} domain={domain_address}",
                format_serial_number(serial_number)
            )),
            ApplicationService::MemoryRead { number, address } => {
                Some(format!("{number} octets at {address:#06x}"))
            }
            ApplicationService::MemoryResponse { address, data }
            | ApplicationService::MemoryWrite { address, data } => Some(format!(
                "{} octets at {address:#06x} = {data:02x?}",
                data.len()
            )),
            ApplicationService::UserMemoryRead { number, address } => {
                Some(format!("{number} octets at {address:#07x}"))
            }
            ApplicationService::UserMemoryResponse { address, data }
            | ApplicationService::UserMemoryWrite { address, data } => Some(format!(
                "{} octets at {address:#07x} = {data:02x?}",
                data.len()
            )),
            ApplicationService::Restart {
                response,
                restart_type,
                data,
            } => Some(format!(
                "type={restart_type}{} data={data:02x?}",
                if *response { " response" } else { "" }
            )),
            // The key itself is never rendered: a bus monitor is the
            // last place a device key should be readable (design spec §10.7).
            ApplicationService::AuthorizeRequest { .. } => Some("key=<redacted>".to_string()),
            ApplicationService::AuthorizeResponse { level } => Some(format!("level={level}")),
            ApplicationService::PropertyValueRead {
                object_index,
                property_id,
                nr_of_elem,
                start_index,
            } => Some(format!(
                "object={object_index} pid={property_id} \
                 nr_of_elem={nr_of_elem} start_index={start_index}"
            )),
            ApplicationService::PropertyValueResponse {
                object_index,
                property_id,
                nr_of_elem,
                start_index,
                data,
            }
            | ApplicationService::PropertyValueWrite {
                object_index,
                property_id,
                nr_of_elem,
                start_index,
                data,
            } => Some(format!(
                "object={object_index} pid={property_id} nr_of_elem={nr_of_elem} \
                 start_index={start_index} data={data:02x?}"
            )),
            ApplicationService::Other { apci, data } => {
                Some(format!("APCI {apci:#06x} data {data:02x?}"))
            }
            _ => None,
        }
    }
}

/// A KNX Serial Number as `MMMM:NNNNNNNN`. RES §4.22.1.2, Figure 61,
/// p. 291 (DPT_SerNum 221.001): two octets of manufacturer code, then
/// four octets *"incremented with each BAU"*.
pub fn format_serial_number(serial_number: &[u8; 6]) -> String {
    format!(
        "{:02X}{:02X}:{:02X}{:02X}{:02X}{:02X}",
        serial_number[0],
        serial_number[1],
        serial_number[2],
        serial_number[3],
        serial_number[4],
        serial_number[5]
    )
}

fn application_service_variant_name(service: &ApplicationService) -> &'static str {
    match service {
        ApplicationService::GroupValueRead => "GroupValueRead",
        ApplicationService::GroupValueResponse(_) => "GroupValueResponse",
        ApplicationService::GroupValueWrite(_) => "GroupValueWrite",
        ApplicationService::DeviceDescriptorRead { .. } => "DeviceDescriptorRead",
        ApplicationService::DeviceDescriptorResponse { .. } => "DeviceDescriptorResponse",
        ApplicationService::IndividualAddressWrite { .. } => "IndividualAddressWrite",
        ApplicationService::IndividualAddressRead => "IndividualAddressRead",
        ApplicationService::IndividualAddressResponse => "IndividualAddressResponse",
        ApplicationService::IndividualAddressSerialNumberRead { .. } => {
            "IndividualAddressSerialNumberRead"
        }
        ApplicationService::IndividualAddressSerialNumberResponse { .. } => {
            "IndividualAddressSerialNumberResponse"
        }
        ApplicationService::IndividualAddressSerialNumberWrite { .. } => {
            "IndividualAddressSerialNumberWrite"
        }
        ApplicationService::DomainAddressWrite { .. } => "DomainAddressWrite",
        ApplicationService::DomainAddressRead => "DomainAddressRead",
        ApplicationService::DomainAddressResponse { .. } => "DomainAddressResponse",
        ApplicationService::DomainAddressSerialNumberRead { .. } => "DomainAddressSerialNumberRead",
        ApplicationService::DomainAddressSerialNumberResponse { .. } => {
            "DomainAddressSerialNumberResponse"
        }
        ApplicationService::DomainAddressSerialNumberWrite { .. } => {
            "DomainAddressSerialNumberWrite"
        }
        ApplicationService::MemoryRead { .. } => "MemoryRead",
        ApplicationService::MemoryResponse { .. } => "MemoryResponse",
        ApplicationService::MemoryWrite { .. } => "MemoryWrite",
        ApplicationService::UserMemoryRead { .. } => "UserMemoryRead",
        ApplicationService::UserMemoryResponse { .. } => "UserMemoryResponse",
        ApplicationService::UserMemoryWrite { .. } => "UserMemoryWrite",
        ApplicationService::Restart { .. } => "Restart",
        ApplicationService::AuthorizeRequest { .. } => "AuthorizeRequest",
        ApplicationService::AuthorizeResponse { .. } => "AuthorizeResponse",
        ApplicationService::PropertyValueRead { .. } => "PropertyValueRead",
        ApplicationService::PropertyValueResponse { .. } => "PropertyValueResponse",
        ApplicationService::PropertyValueWrite { .. } => "PropertyValueWrite",
        ApplicationService::NoApplicationPdu => "NoApplicationPdu",
        ApplicationService::Other { .. } => "Other",
    }
}

fn encode_group_value(value: &GroupValue) -> (u8, &[u8]) {
    match value {
        // A `Short` whose value needs more than six bits does not fit the
        // inline APCI-octet field: `short_apci`'s own two low bits share
        // that octet with the top two bits of `inline6` (see the encoding
        // above), so a `Short(v)` with `v > 0x3F` would overwrite them and
        // change which `A_GroupValue_*` service the frame decodes as.
        // Application Layer v02.01.01 AS §3.1.3 already draws the line at
        // six bits ("Values that only consist of 6 bits or less have the
        // following optimized A_GroupValue_Write-PDU format"), so an
        // out-of-range `Short` is promoted to the one-octet `Bytes` form
        // instead — the value survives, and the APCI is not touched.
        GroupValue::Short(v) if *v > 0x3F => (0, std::slice::from_ref(v)),
        GroupValue::Short(inline6) => (*inline6, &[]),
        GroupValue::Bytes(bytes) => (0, bytes),
    }
}

/// Encodes octet 6 — bit table and citation on `Tpci`'s doc comment.
/// Rejects (rather than masking) a `seq` above 15: the Transport Control
/// Field's SeqNo is 4 bits, and `& 0x0F` would silently turn an
/// out-of-range value into a different, valid one — precisely the failure
/// mode `CemiError::InvalidSequenceNumber` exists to name instead.
fn encode_tpci(tpci: &Tpci) -> Result<u8, CemiError> {
    match tpci {
        Tpci::UnnumberedData => Ok(0x00),
        Tpci::NumberedData { seq } => Ok(0x40 | (checked_seq(*seq)? << 2)),
        Tpci::Connect => Ok(0x80),
        Tpci::Disconnect => Ok(0x81),
        Tpci::Ack { seq } => Ok(0xC2 | (checked_seq(*seq)? << 2)),
        Tpci::Nak { seq } => Ok(0xC3 | (checked_seq(*seq)? << 2)),
        // Data-shaped (bit 7 clear): mask bits 1-0 off before returning —
        // `encode_l_data` ORs the short-APCI's/`Other`'s own top two bits
        // into this octet next, and those two bits are the only field that
        // is allowed to put anything there (see `Tpci::Unknown`'s doc
        // comment). A caller-supplied `Unknown(0x06)` and `Unknown(0x04)`
        // must produce the same encoded octet once the APCI bits are ORed
        // in, or the service the frame decodes back as would depend on
        // bits that were never transport's. Control-shaped (bit 7 set)
        // octets have no such foreign field to protect and keep all eight
        // bits, as before.
        Tpci::Unknown(octet) => Ok(if *octet & 0x80 == 0 {
            octet & 0xFC
        } else {
            *octet
        }),
    }
}

/// `seq` must fit the Transport Control Field's 4-bit SeqNo (0-15) — see
/// `encode_tpci`'s doc comment for why this rejects rather than masks.
fn checked_seq(seq: u8) -> Result<u8, CemiError> {
    if seq > 0x0F {
        Err(CemiError::InvalidSequenceNumber(seq))
    } else {
        Ok(seq)
    }
}

/// Assembles the fixed cEMI header around an already-encoded NPDU (TPCI
/// octet, then APCI octet and data if the TPDU carries one) and derives
/// `L` from its length (EMI_IMI v01.04.02 AS §4.1.5.3.2: `L` is the NPDU
/// length minus one; `npdu` is never empty, so the subtraction itself
/// never underflows). The result still has to fit in the one octet `L`
/// is — `npdu.len() - 1` above 255 is rejected with `CemiError::
/// NpduTooLong` rather than truncated, which is the only other option
/// `as u8` has.
fn finish_l_data(
    message_code: u8,
    ctrl1: u8,
    ctrl2: u8,
    source_raw: u16,
    dest_raw: u16,
    npdu: &[u8],
) -> Result<Vec<u8>, CemiError> {
    let l_octet =
        u8::try_from(npdu.len() - 1).map_err(|_| CemiError::NpduTooLong { got: npdu.len() })?;
    let mut buf = Vec::with_capacity(9 + npdu.len());
    buf.push(message_code);
    buf.push(0x00); // additional info length
    buf.push(ctrl1);
    buf.push(ctrl2);
    buf.extend_from_slice(&source_raw.to_be_bytes());
    buf.extend_from_slice(&dest_raw.to_be_bytes());
    buf.push(l_octet);
    buf.extend_from_slice(npdu);
    Ok(buf)
}

#[cfg(test)]
mod tests {
    use super::*;
    use knx_core::{GroupAddress, GroupValue, IndividualAddress};

    /// `[D]` TL clause 2, Figure 3, p. 6 of 38:
    /// *"T_Data_Broadcast-PDU (destination_address = 0)"* against
    /// *"T_Data_Group-PDU (destination_address <> 0)"*.
    ///
    /// Asserted against a literal rather than against
    /// [`BROADCAST_DESTINATION`] itself: every other use of the constant —
    /// the client's two broadcast primitives and the simulator's
    /// `handle_broadcast` — compares it against itself, so the whole suite
    /// stays green on any value at all. This is the one place the number
    /// is checked against the Standard's.
    #[test]
    fn broadcast_destination_is_group_address_zero() {
        assert_eq!(
            BROADCAST_DESTINATION,
            Destination::Group(GroupAddress::from_raw(0))
        );
        let Destination::Group(group) = BROADCAST_DESTINATION else {
            panic!("a broadcast is group-addressed: AT=1 in Figure 3's own table");
        };
        assert_eq!(group.raw(), 0);
    }

    /// A minimal, hand-built `L_Data.ind` carrying `A_GroupValue_Write`
    /// with a 6-bit inline value (e.g. DPT-1 "on"): message code 29h, no
    /// additional info, Ctrl1/Ctrl2 for a standard group-addressed frame,
    /// source raw 0x1101, destination group address raw 0x0903, L=1,
    /// TPCI/APCI = GroupValueWrite (EMI_IMI v01.04.02 AS §4.1.3.2 Table 1 /
    /// Application Layer v02.01.01 AS §2.2 Table 1) with inline data = 1
    /// (on). The raw address values are arbitrary test fixtures, not
    /// meant to format to any particular `main/middle/sub` string.
    fn write_on_frame() -> Vec<u8> {
        vec![
            0x29, // L_Data.ind
            0x00, // no additional information
            0xBC, // Ctrl1: standard frame, no repeat, ack requested — exact
            // flag values do not matter to this decoder, only the
            // message code / Ctrl2 / addresses / TPCI-APCI do
            0xE0, // Ctrl2: AT=1 (group), hop count 6, EFF 0000
            0x11, 0x01, // source, raw 0x1101
            0x09, 0x03, // destination group address, raw 0x0903
            0x01, // L = 1 (one NPDU octet after TPCI)
            0x00, // TPCI: UDT, APCI high bits 00
            0x81, // APCI low bits 10 (Write) | data 000001 (value 1)
        ]
    }

    #[test]
    fn decodes_group_value_write_with_inline_data() {
        let frame = decode_l_data(&write_on_frame()).unwrap();
        assert_eq!(frame.kind, LDataMessageKind::Indication);
        assert_eq!(frame.source, IndividualAddress::from_raw(0x1101));
        assert_eq!(
            frame.destination,
            Destination::Group(GroupAddress::from_raw(0x0903))
        );
        assert_eq!(
            frame.service,
            ApplicationService::GroupValueWrite(GroupValue::Short(0x01))
        );
    }

    #[test]
    fn decodes_group_value_read() {
        let mut bytes = write_on_frame();
        let len = bytes.len();
        bytes[len - 2] = 0x00; // TPCI high bits 00
        bytes[len - 1] = 0x00; // APCI low bits 00 (Read), no data
        let frame = decode_l_data(&bytes).unwrap();
        assert_eq!(frame.service, ApplicationService::GroupValueRead);
    }

    #[test]
    fn decodes_group_value_response_with_a_full_data_byte() {
        let mut bytes = write_on_frame();
        let len = bytes.len();
        bytes[len - 3] = 0x02; // L = 2 (TPCI/APCI-low octet + one data octet)
        bytes[len - 2] = 0x00; // TPCI high bits 00
        bytes[len - 1] = 0x40; // APCI low bits 01 (Response), no inline data
        bytes.push(0x2A); // the one data octet: 42
        let frame = decode_l_data(&bytes).unwrap();
        assert_eq!(
            frame.service,
            ApplicationService::GroupValueResponse(GroupValue::Bytes(vec![0x2A]))
        );
    }

    #[test]
    fn destination_individual_when_at_bit_clear() {
        let mut bytes = write_on_frame();
        bytes[3] = 0x60; // Ctrl2: AT=0 (individual), hop count 6
        let frame = decode_l_data(&bytes).unwrap();
        assert_eq!(
            frame.destination,
            Destination::Individual(IndividualAddress::from_raw(0x0903))
        );
    }

    #[test]
    fn confirmation_carries_the_error_flag() {
        let mut bytes = write_on_frame();
        bytes[0] = 0x2E; // L_Data.con
        bytes[2] = 0xBD; // Ctrl1 with the Confirm (C, lsb) bit set
        let frame = decode_l_data(&bytes).unwrap();
        assert_eq!(frame.kind, LDataMessageKind::Confirmation { error: true });
    }

    #[test]
    fn unsupported_message_code_is_reported_not_panicked() {
        let mut bytes = write_on_frame();
        bytes[0] = 0xFC; // not a defined cEMI message code
        let err = decode_l_data(&bytes).unwrap_err();
        assert_eq!(err, CemiError::UnsupportedMessageCode(0xFC));
    }

    #[test]
    fn decode_rejects_short_buffer() {
        let err = decode_l_data(&[0x29]).unwrap_err();
        assert_eq!(err, CemiError::TooShort { needed: 2, got: 1 });
    }

    /// `L_Data.req` (message code 11h) form of `write_on_frame` — same
    /// Ctrl1/Ctrl2/addresses/TPCI-APCI, sent rather than received. Ctrl1
    /// 0xBC and Ctrl2's hop-count-6/AT-bit form here match the incoming
    /// fixture above; RESEARCH.md notes no reason cEMI would ask for a
    /// different default outbound.
    fn write_on_request() -> Vec<u8> {
        let mut bytes = write_on_frame();
        bytes[0] = L_DATA_REQ;
        bytes
    }

    #[test]
    fn encode_l_data_matches_hand_built_group_value_write_request() {
        let frame = LDataFrame {
            kind: LDataMessageKind::Request,
            source: IndividualAddress::from_raw(0x1101),
            destination: Destination::Group(GroupAddress::from_raw(0x0903)),
            transport: Tpci::UnnumberedData,
            service: ApplicationService::GroupValueWrite(GroupValue::Short(0x01)),
        };
        assert_eq!(encode_l_data(&frame).unwrap(), write_on_request());
    }

    #[test]
    fn encode_l_data_round_trips_through_decode() {
        let frame = LDataFrame {
            kind: LDataMessageKind::Request,
            source: IndividualAddress::from_raw(0x1101),
            destination: Destination::Group(GroupAddress::from_raw(0x0903)),
            transport: Tpci::UnnumberedData,
            service: ApplicationService::GroupValueWrite(GroupValue::Bytes(vec![0x2A, 0x99])),
        };
        let encoded = encode_l_data(&frame).unwrap();
        assert_eq!(decode_l_data(&encoded).unwrap(), frame);
    }

    /// The `L` octet at `finish_l_data` (EMI_IMI v01.04.02 AS §4.1.5.3.2)
    /// is one octet holding `npdu.len() - 1`, so the largest NPDU it can
    /// name is 256 octets (`L` = 255, `u8::MAX`). An NPDU one octet longer
    /// than that has no `L` value to hold it — protects against I1
    /// (`(npdu.len() - 1) as u8` used to wrap silently at this boundary
    /// instead of erroring).
    #[test]
    fn encode_l_data_accepts_npdu_at_the_256_octet_l_boundary() {
        // NPDU = TPCI octet + APCI-low octet + 254 data octets = 256.
        let frame = LDataFrame {
            kind: LDataMessageKind::Request,
            source: IndividualAddress::from_raw(0x1101),
            destination: Destination::Group(GroupAddress::from_raw(0x0903)),
            transport: Tpci::UnnumberedData,
            service: ApplicationService::GroupValueWrite(GroupValue::Bytes(vec![0xAA; 254])),
        };
        let encoded = encode_l_data(&frame).unwrap();
        assert_eq!(encoded[8], 255); // L octet: 256 - 1
        assert_eq!(decode_l_data(&encoded).unwrap(), frame);
    }

    #[test]
    fn encode_l_data_rejects_npdu_one_octet_past_the_l_boundary() {
        // NPDU = TPCI octet + APCI-low octet + 255 data octets = 257 —
        // `npdu.len() - 1` = 256, which does not fit in the one-octet `L`.
        let frame = LDataFrame {
            kind: LDataMessageKind::Request,
            source: IndividualAddress::from_raw(0x1101),
            destination: Destination::Group(GroupAddress::from_raw(0x0903)),
            transport: Tpci::UnnumberedData,
            service: ApplicationService::GroupValueWrite(GroupValue::Bytes(vec![0xAA; 255])),
        };
        assert_eq!(
            encode_l_data(&frame),
            Err(CemiError::NpduTooLong { got: 257 })
        );
    }

    #[test]
    fn encode_l_data_round_trips_group_value_read() {
        let frame = LDataFrame {
            kind: LDataMessageKind::Request,
            source: IndividualAddress::from_raw(0x0000),
            destination: Destination::Group(GroupAddress::from_raw(0x0903)),
            transport: Tpci::UnnumberedData,
            service: ApplicationService::GroupValueRead,
        };
        let encoded = encode_l_data(&frame).unwrap();
        assert_eq!(decode_l_data(&encoded).unwrap(), frame);
    }

    #[test]
    fn encode_l_data_round_trips_individual_destination() {
        let frame = LDataFrame {
            kind: LDataMessageKind::Request,
            source: IndividualAddress::from_raw(0x1101),
            destination: Destination::Individual(IndividualAddress::from_raw(0x1102)),
            transport: Tpci::UnnumberedData,
            service: ApplicationService::GroupValueWrite(GroupValue::Short(0x00)),
        };
        let encoded = encode_l_data(&frame).unwrap();
        assert_eq!(decode_l_data(&encoded).unwrap(), frame);
    }

    #[test]
    fn encode_l_data_round_trips_other_apci() {
        let frame = LDataFrame {
            kind: LDataMessageKind::Request,
            source: IndividualAddress::from_raw(0x1101),
            destination: Destination::Group(GroupAddress::from_raw(0x0903)),
            transport: Tpci::UnnumberedData,
            service: ApplicationService::Other {
                apci: 0x03C0,
                data: vec![0xAB, 0xCD],
            },
        };
        let encoded = encode_l_data(&frame).unwrap();
        assert_eq!(decode_l_data(&encoded).unwrap(), frame);
    }

    #[test]
    fn unknown_apci_is_reported_as_other_not_dropped() {
        let mut bytes = write_on_frame();
        let len = bytes.len();
        bytes[len - 2] = 0x03; // TPCI high bits 11 (not one of the three group
                               // services this cycle interprets)
        bytes[len - 1] = 0xC0;
        let frame = decode_l_data(&bytes).unwrap();
        match frame.service {
            ApplicationService::Other { .. } => {}
            other => panic!("expected Other, got {other:?}"),
        }
    }

    /// Regression for the APCI-corruption bug (spec E4-D3): before the
    /// `encode_group_value` fix, `Short(0x40)` set `apci_lo`'s top two bits
    /// (meant for `short_apci`) from `inline6`'s own top bit, so a
    /// `GroupValueWrite` decoded back as `short_apci = 0b0011` —
    /// `ApplicationService::Other`, with the value gone. `0x40` is the
    /// smallest value that does not fit the six inline bits (`0x3F` is the
    /// largest that does), so it is the minimal case that exercises the
    /// bug.
    #[test]
    fn encode_promotes_out_of_range_short_instead_of_corrupting_apci() {
        let frame = LDataFrame {
            kind: LDataMessageKind::Request,
            source: IndividualAddress::from_raw(0x1101),
            destination: Destination::Group(GroupAddress::from_raw(0x0903)),
            transport: Tpci::UnnumberedData,
            service: ApplicationService::GroupValueWrite(GroupValue::Short(0x40)),
        };
        let encoded = encode_l_data(&frame).unwrap();
        let decoded = decode_l_data(&encoded).unwrap();
        assert_eq!(
            decoded.service,
            ApplicationService::GroupValueWrite(GroupValue::Bytes(vec![0x40]))
        );
    }

    // -- Tpci (spec T17) -----------------------------------------------

    /// Builds a minimal individually-addressed frame with the given
    /// `transport`/`service`, encodes it, decodes the result, and checks
    /// both that the decoded frame equals the original and that
    /// re-encoding it produces byte-identical output — the round-trip
    /// property the brief asks every `Tpci` variant to have.
    fn assert_tpci_round_trips(transport: Tpci, service: ApplicationService) {
        let frame = LDataFrame {
            kind: LDataMessageKind::Request,
            source: IndividualAddress::from_raw(0x1101),
            destination: Destination::Individual(IndividualAddress::from_raw(0x1102)),
            transport,
            service,
        };
        let encoded = encode_l_data(&frame).expect("a valid transport/service pairing encodes");
        let decoded = decode_l_data(&encoded).expect("a frame this function built itself decodes");
        assert_eq!(decoded, frame);
        assert_eq!(encode_l_data(&decoded).unwrap(), encoded);
    }

    #[test]
    fn every_tpci_variant_round_trips_byte_identical() {
        assert_tpci_round_trips(Tpci::UnnumberedData, ApplicationService::GroupValueRead);
        assert_tpci_round_trips(
            Tpci::NumberedData { seq: 7 },
            ApplicationService::DeviceDescriptorRead { descriptor_type: 0 },
        );
        assert_tpci_round_trips(Tpci::Connect, ApplicationService::NoApplicationPdu);
        assert_tpci_round_trips(Tpci::Disconnect, ApplicationService::NoApplicationPdu);
        assert_tpci_round_trips(Tpci::Ack { seq: 3 }, ApplicationService::NoApplicationPdu);
        assert_tpci_round_trips(Tpci::Nak { seq: 15 }, ApplicationService::NoApplicationPdu);
    }

    /// A hand-built `T_Connect` fixture (EMI_IMI v01.04.02 AS §4.1.5.3.2
    /// fixed layout, TPCI octet 0x80 per `Tpci`'s bit table): the frame
    /// ends at the TPCI octet, so decoding it must not demand the APCI
    /// octet the data-PDU path requires — this is Global Constraint 1's
    /// length-arithmetic change, exercised directly.
    fn connect_frame() -> Vec<u8> {
        vec![
            0x29, // L_Data.ind
            0x00, // no additional information
            0xBC, // Ctrl1
            0x60, // Ctrl2: AT=0 (individual), hop count 6
            0x11, 0x01, // source, raw 0x1101
            0x11, 0x02, // destination individual address, raw 0x1102
            0x00, // L = 0 (TPDU is the TPCI octet alone)
            0x80, // TPCI: T_Connect
        ]
    }

    #[test]
    fn t_connect_decodes_with_no_application_pdu_and_no_length_error() {
        let frame =
            decode_l_data(&connect_frame()).expect("a control PDU needs only its TPCI octet");
        assert_eq!(frame.transport, Tpci::Connect);
        assert_eq!(frame.service, ApplicationService::NoApplicationPdu);
    }

    #[test]
    fn encode_l_data_matches_hand_built_t_connect_indication() {
        let frame = LDataFrame {
            kind: LDataMessageKind::Indication,
            source: IndividualAddress::from_raw(0x1101),
            destination: Destination::Individual(IndividualAddress::from_raw(0x1102)),
            transport: Tpci::Connect,
            service: ApplicationService::NoApplicationPdu,
        };
        assert_eq!(encode_l_data(&frame).unwrap(), connect_frame());
    }

    /// `T_Data_Tag_Group` (octet 6 = `0x04`) is not one of the six named
    /// `Tpci` variants — it must decode as `Unknown(0x04)`, not silently
    /// folded into `UnnumberedData` just because its top two bits match
    /// (Global Constraint 2). Its low two bits are `00`, so it is
    /// data-shaped (bit 7 clear) and the frame's real APCI (`GroupValueWrite`
    /// here, from `write_on_frame`'s unchanged APCI-low octet) still
    /// decodes underneath it — `T_Data_Tag_Group` genuinely shares
    /// `T_Data_Group`'s PDU shape, it is only the *transport* semantics
    /// (message tagging) this cycle does not model.
    #[test]
    fn tag_group_tpci_decodes_as_unknown_transport_not_unnumbered_data() {
        let mut bytes = write_on_frame();
        let len = bytes.len();
        bytes[len - 2] = 0x04;
        let frame = decode_l_data(&bytes).unwrap();
        assert_eq!(frame.transport, Tpci::Unknown(0x04));
        assert_ne!(frame.transport, Tpci::UnnumberedData);
        assert_eq!(
            frame.service,
            ApplicationService::GroupValueWrite(GroupValue::Short(0x01))
        );
    }

    /// A data-shaped `Unknown` whose raw octet already has bits 1-0 clear
    /// (Fix round 2, finding 14b): `assert_tpci_round_trips` demands both
    /// `decoded == frame` and byte-identical re-encoding, so this is the
    /// direct regression test the re-review named — it would have failed
    /// before this fix masked the short-APCI's stolen bits out of
    /// `Unknown`'s stored value.
    #[test]
    fn data_shaped_unknown_tpci_round_trips_against_every_kind_of_service() {
        assert_tpci_round_trips(
            Tpci::Unknown(0x04),
            ApplicationService::DeviceDescriptorRead { descriptor_type: 0 },
        );
        assert_tpci_round_trips(
            Tpci::Unknown(0x04),
            ApplicationService::GroupValueWrite(GroupValue::Short(0x01)),
        );
        assert_tpci_round_trips(
            Tpci::Unknown(0x04),
            ApplicationService::Other {
                apci: 0x00C0,
                data: vec![],
            },
        );
    }

    /// Reproduces the re-review's three findings directly, using a
    /// data-shaped `Unknown` whose own raw octet has bits 1-0 *set* —
    /// exactly the shape that leaked into the APCI before this fix. Bits
    /// 1-0 never belonged to transport (`Tpci::Unknown`'s doc comment), so
    /// the encoder masks them off rather than reject: the frame decodes
    /// with `Unknown(0x04)` (the masked, canonical form) and the caller's
    /// actual service — not the different service the review's repro
    /// (`Unknown(0x06)` + `GroupValueWrite` decoding as `Other { apci:
    /// 0x0281 }`) demonstrated.
    #[test]
    fn data_shaped_unknown_tpci_with_nonzero_low_bits_does_not_corrupt_the_apci() {
        let frame = LDataFrame {
            kind: LDataMessageKind::Request,
            source: IndividualAddress::from_raw(0x1101),
            destination: Destination::Individual(IndividualAddress::from_raw(0x1102)),
            transport: Tpci::Unknown(0x06),
            service: ApplicationService::GroupValueWrite(GroupValue::Short(0x01)),
        };
        let decoded = decode_l_data(&encode_l_data(&frame).unwrap()).unwrap();
        assert_eq!(decoded.transport, Tpci::Unknown(0x04));
        assert_eq!(
            decoded.service,
            ApplicationService::GroupValueWrite(GroupValue::Short(0x01))
        );

        // The review's third repro (`Unknown(0x05)` + `Other { apci:
        // 0x0040 }` decoding back as `0x0140`) used an `apci` whose 4-bit
        // short-APCI selector (top 2 bits 00, bottom 2 bits 01) collides
        // with `GroupValueResponse`'s reserved pattern independent of this
        // bug — that collision is pre-existing, documented behavior of the
        // 4-bit short-APCI space, not finding 14. `0x00C0`'s selector
        // (0b0011) is not one of the five reserved patterns, so it
        // isolates the `Unknown`-bit-leak this test exists to catch.
        let frame = LDataFrame {
            transport: Tpci::Unknown(0x05),
            service: ApplicationService::Other {
                apci: 0x00C0,
                data: vec![],
            },
            ..frame
        };
        let decoded = decode_l_data(&encode_l_data(&frame).unwrap()).unwrap();
        assert_eq!(decoded.transport, Tpci::Unknown(0x04));
        assert_eq!(
            decoded.service,
            ApplicationService::Other {
                apci: 0x00C0,
                data: vec![],
            }
        );
    }

    /// NOTE 1 under the Transport Control Field figure reserves this
    /// encoding. Its top bit is set, so `is_control_pdu` treats it as
    /// control-shaped (a 1-octet TPDU, same family as `Connect`/
    /// `Disconnect`/`Ack`/`Nak`) — the frame it is spliced into here still
    /// carries `write_on_frame`'s trailing APCI/data octet, which a
    /// control-shaped TPDU has no field for, so this is rejected as
    /// `UnexpectedControlPduData`, not coerced into `Connect` or
    /// `Disconnect`.
    #[test]
    fn reserved_tpci_bf_is_treated_as_unknown_control_pdu_not_coerced() {
        let mut bytes = write_on_frame();
        let len = bytes.len();
        bytes[len - 2] = 0xBF;
        let err = decode_l_data(&bytes).unwrap_err();
        assert_eq!(
            err,
            CemiError::UnexpectedControlPduData {
                tpci: 0xBF,
                extra_octets: 1,
            }
        );
    }

    /// A minimal, well-formed `0xBF`-TPCI frame (no trailing octets, `L =
    /// 0`, same shape as `connect_frame`): decodes as `Unknown(0xBF)` with
    /// `NoApplicationPdu`, and round-trips byte-identical — the reserved
    /// encoding is preserved raw, not treated as an error just because it
    /// is reserved (Global Constraint 2).
    #[test]
    fn reserved_tpci_bf_round_trips_as_unknown_control_pdu_when_well_formed() {
        assert_tpci_round_trips(Tpci::Unknown(0xBF), ApplicationService::NoApplicationPdu);
    }

    /// The exact reproduction from the task-2 review, finding 3: a
    /// `T_Connect` (`L = 3`) with three octets following its TPCI octet.
    /// `Tpci::Connect`'s TPDU has no field for them — rejected, naming how
    /// many followed, not silently dropped (Global Constraint 2).
    #[test]
    fn control_pdu_with_trailing_octets_is_rejected_naming_the_count() {
        let bytes = vec![
            0x29, // L_Data.ind
            0x00, // no additional information
            0xBC, // Ctrl1
            0x60, // Ctrl2: AT=0 (individual), hop count 6
            0x11, 0x01, // source, raw 0x1101
            0x11, 0x02, // destination individual address, raw 0x1102
            0x03, // L = 3 (three unexpected trailing octets)
            0x80, // TPCI: T_Connect
            0xAA, 0xBB, 0xCC,
        ];
        let err = decode_l_data(&bytes).unwrap_err();
        assert_eq!(
            err,
            CemiError::UnexpectedControlPduData {
                tpci: 0x80,
                extra_octets: 3,
            }
        );
    }

    /// Same finding, the `T_ACK` shape (`L = 1`, one trailing octet) —
    /// confirms the check is not special-cased to `Connect`.
    #[test]
    fn t_ack_with_one_trailing_octet_is_rejected_naming_the_count() {
        let bytes = vec![
            0x29, 0x00, 0xBC, 0x60, 0x11, 0x01, 0x11, 0x02, 0x01, // L = 1
            0xC2, // TPCI: T_ACK seq=0
            0xDD, // unexpected trailing octet
        ];
        let err = decode_l_data(&bytes).unwrap_err();
        assert_eq!(
            err,
            CemiError::UnexpectedControlPduData {
                tpci: 0xC2,
                extra_octets: 1,
            }
        );
    }

    /// Finding 3b/4: pairing a control `Tpci` with an application service
    /// (or vice versa) does not encode a malformed frame — it is rejected,
    /// naming both halves of the bad pair.
    #[test]
    fn control_transport_paired_with_application_service_is_rejected_at_encode() {
        let frame = LDataFrame {
            kind: LDataMessageKind::Request,
            source: IndividualAddress::from_raw(0x1101),
            destination: Destination::Group(GroupAddress::from_raw(0x0903)),
            transport: Tpci::Connect,
            service: ApplicationService::GroupValueWrite(GroupValue::Short(1)),
        };
        assert_eq!(
            encode_l_data(&frame).unwrap_err(),
            CemiError::MismatchedTransport {
                transport: "Connect",
                service: "GroupValueWrite",
            }
        );
    }

    /// The other direction of the same invariant: a data-shaped `Tpci`
    /// paired with `NoApplicationPdu` is just as unencodable (the review's
    /// `UnnumberedData` + `NoApplicationPdu` example, which used to encode
    /// `[.. 00 00]` — a frame this library could not decode back).
    #[test]
    fn data_transport_paired_with_no_application_pdu_is_rejected_at_encode() {
        let frame = LDataFrame {
            kind: LDataMessageKind::Request,
            source: IndividualAddress::from_raw(0x1101),
            destination: Destination::Individual(IndividualAddress::from_raw(0x1102)),
            transport: Tpci::UnnumberedData,
            service: ApplicationService::NoApplicationPdu,
        };
        assert_eq!(
            encode_l_data(&frame).unwrap_err(),
            CemiError::MismatchedTransport {
                transport: "UnnumberedData",
                service: "NoApplicationPdu",
            }
        );
    }

    /// Finding 1: a `seq` above 15 does not fit the Transport Control
    /// Field's 4-bit SeqNo. Rejected, not masked into a different, valid
    /// sequence number (`NumberedData { seq: 16 }` used to encode
    /// byte-identical to `seq: 0`; `Ack { seq: 200 }` to `Ack { seq: 8 }`).
    #[test]
    fn sequence_number_above_15_is_rejected_not_masked() {
        let mut frame = LDataFrame {
            kind: LDataMessageKind::Request,
            source: IndividualAddress::from_raw(0x1101),
            destination: Destination::Individual(IndividualAddress::from_raw(0x1102)),
            transport: Tpci::NumberedData { seq: 16 },
            service: ApplicationService::DeviceDescriptorRead { descriptor_type: 0 },
        };
        assert_eq!(
            encode_l_data(&frame).unwrap_err(),
            CemiError::InvalidSequenceNumber(16)
        );

        frame.transport = Tpci::Ack { seq: 200 };
        frame.service = ApplicationService::NoApplicationPdu;
        assert_eq!(
            encode_l_data(&frame).unwrap_err(),
            CemiError::InvalidSequenceNumber(200)
        );

        frame.transport = Tpci::Nak { seq: 16 };
        assert_eq!(
            encode_l_data(&frame).unwrap_err(),
            CemiError::InvalidSequenceNumber(16)
        );
    }

    /// Finding 5 (should-fix): the 4-bit SeqNo boundary, on every
    /// seq-carrying `Tpci`, both ends.
    #[test]
    fn sequence_number_boundaries_round_trip_for_every_numbered_tpci() {
        for seq in [0u8, 15] {
            assert_tpci_round_trips(
                Tpci::NumberedData { seq },
                ApplicationService::DeviceDescriptorRead { descriptor_type: 0 },
            );
            assert_tpci_round_trips(Tpci::Ack { seq }, ApplicationService::NoApplicationPdu);
            assert_tpci_round_trips(Tpci::Nak { seq }, ApplicationService::NoApplicationPdu);
        }
    }

    // -- A_DeviceDescriptor_Read/Response (spec T17) --------------------

    #[test]
    fn device_descriptor_read_round_trips_with_its_sequence_number_intact() {
        assert_tpci_round_trips(
            Tpci::NumberedData { seq: 5 },
            ApplicationService::DeviceDescriptorRead { descriptor_type: 0 },
        );
    }

    /// A hand-built `A_DeviceDescriptor_Response` (Application Layer
    /// v02.01.01 AS §3.4.2.1) on a connected, numbered link (`seq` 0):
    /// TPCI `0x40 | seq<<2` OR'd with the APCI-high bits `11` (`0x43`),
    /// APCI-low `0x40` (Response marker `01` in bits 7-6, descriptor_type
    /// 0 in bits 5-0), followed by a two-octet Mask Version
    /// (`07B0`, an arbitrary test value, not any real device's).
    fn device_descriptor_response_frame() -> Vec<u8> {
        vec![
            0x29, // L_Data.ind
            0x00, // no additional information
            0xBC, // Ctrl1
            0x60, // Ctrl2: AT=0 (individual), hop count 6
            0x11, 0x01, // source, raw 0x1101
            0x11, 0x02, // destination individual address, raw 0x1102
            0x03, // L = 3 (TPCI/APCI-low octet + two data octets)
            0x43, // TPCI: T_Data_Connected seq=0, APCI-high bits 11
            0x40, // APCI-low: Response (01) | descriptor_type 0
            0x07, 0xB0, // Mask Version data
        ]
    }

    #[test]
    fn device_descriptor_response_fixture_decodes_type_and_data() {
        let frame = decode_l_data(&device_descriptor_response_frame()).unwrap();
        assert_eq!(frame.transport, Tpci::NumberedData { seq: 0 });
        assert_eq!(
            frame.service,
            ApplicationService::DeviceDescriptorResponse {
                descriptor_type: 0,
                data: vec![0x07, 0xB0],
            }
        );
    }

    /// An `A_DeviceDescriptor_Read`-shaped APCI with an unexpected
    /// trailing data octet does not match the well-formed (always
    /// dataless, Application Layer v02.01.01 AS §3.4.2.1) PDU shape, so it
    /// must fall to `Other` and keep the octet, not silently drop it
    /// (CLAUDE.md: never silently discard).
    #[test]
    fn device_descriptor_read_with_unexpected_data_falls_to_other_not_dropped() {
        let bytes = vec![
            0x29, // L_Data.ind
            0x00, // no additional information
            0xBC, // Ctrl1
            0x60, // Ctrl2: AT=0 (individual), hop count 6
            0x11, 0x01, // source, raw 0x1101
            0x11, 0x02, // destination individual address, raw 0x1102
            0x02, // L = 2 (TPCI/APCI-low octet + one unexpected data octet)
            0x43, // TPCI: T_Data_Connected seq=0, APCI-high bits 11
            0x00, // APCI-low: Read (00) | descriptor_type 0
            0x07, // unexpected trailing octet
        ];
        let frame = decode_l_data(&bytes).unwrap();
        match frame.service {
            ApplicationService::Other { apci, ref data } => {
                assert_eq!(apci, 0x0300);
                assert_eq!(data, &[0x07]);
            }
            other => panic!("expected Other, got {other:?}"),
        }
    }

    /// A 10-bit APCI this cycle does not interpret (`short_apci = 0b0110`,
    /// `A_ADC_Read`) still lands in `Other` with its octets intact — same
    /// guarantee as the pre-T17 `unknown_apci_is_reported_as_other_not_
    /// dropped`, re-affirmed after `Tpci` was split out of the stored
    /// `apci` value.
    ///
    /// T30 moved this test off `0b0101` (`0x0140`): that APCI is
    /// `A_IndividualAddress_Response`, which is now a typed variant. The
    /// guarantee under test is unchanged — it just needs an APCI that is
    /// still genuinely uninterpreted, and `A_ADC_Read` is one.
    #[test]
    fn unknown_ten_bit_apci_lands_in_other_with_octets_intact() {
        let mut bytes = write_on_frame();
        let len = bytes.len();
        bytes[len - 2] = 0x01; // TPCI: UnnumberedData, APCI-high bits 01
        bytes[len - 1] = 0x80; // APCI-low bits 10 -> short_apci = 0b0110
        let frame = decode_l_data(&bytes).unwrap();
        assert_eq!(
            frame.service,
            ApplicationService::Other {
                apci: 0x0180,
                data: vec![],
            }
        );
    }

    /// Finding 2: a `descriptor_type` above `0x3F` does not fit the six
    /// bits `A_DeviceDescriptor_Read`/`Response` give it (Figures 36/38,
    /// octet 7 bits 5-0) — those bits share their octet with the two bits
    /// that select `Read` (`00`) vs. `Response` (`01`), so an unguarded
    /// overflow would silently change which service the frame decodes as
    /// (`DeviceDescriptorRead { descriptor_type: 0x40 }` used to encode and
    /// decode back as `DeviceDescriptorResponse { descriptor_type: 0 }`).
    /// Rejected, not silently reinterpreted, on both variants.
    #[test]
    fn descriptor_type_above_0x3f_is_rejected_not_flipped_into_a_different_service() {
        let read_frame = LDataFrame {
            kind: LDataMessageKind::Request,
            source: IndividualAddress::from_raw(0x1101),
            destination: Destination::Individual(IndividualAddress::from_raw(0x1102)),
            transport: Tpci::UnnumberedData,
            service: ApplicationService::DeviceDescriptorRead {
                descriptor_type: 0x40,
            },
        };
        assert_eq!(
            encode_l_data(&read_frame).unwrap_err(),
            CemiError::InvalidDescriptorType(0x40)
        );

        let response_frame = LDataFrame {
            kind: LDataMessageKind::Request,
            source: IndividualAddress::from_raw(0x1101),
            destination: Destination::Individual(IndividualAddress::from_raw(0x1102)),
            transport: Tpci::UnnumberedData,
            service: ApplicationService::DeviceDescriptorResponse {
                descriptor_type: 0xFF,
                data: vec![1, 2],
            },
        };
        assert_eq!(
            encode_l_data(&response_frame).unwrap_err(),
            CemiError::InvalidDescriptorType(0xFF)
        );
    }

    // ---------------------------------------------------------------
    // The §6.6 management services (T30 phase 2). Every test here is
    // pure codec: no socket, no device, no bus.
    // ---------------------------------------------------------------

    /// A connection-oriented management frame to a device, which is how
    /// every one of these services actually travels (spec §7.1: the load
    /// procedure is connection oriented).
    fn mgmt(service: ApplicationService) -> LDataFrame {
        LDataFrame {
            kind: LDataMessageKind::Request,
            source: IndividualAddress::from_raw(0x1101),
            destination: Destination::Individual(IndividualAddress::from_raw(0x1118)),
            transport: Tpci::NumberedData { seq: 0 },
            service,
        }
    }

    /// The NPDU (TPCI octet onwards), which is where all the interesting
    /// arithmetic lives.
    fn npdu_of(service: ApplicationService) -> Vec<u8> {
        let bytes = encode_l_data(&mgmt(service)).unwrap();
        // message code, add-info length, Ctrl1, Ctrl2, source (2),
        // destination (2), L — nine octets before the NPDU.
        bytes[9..].to_vec()
    }

    fn round_trip(service: ApplicationService) -> ApplicationService {
        let bytes = encode_l_data(&mgmt(service)).unwrap();
        decode_l_data(&bytes).unwrap().service
    }

    /// The APCI column of Application Layer v02.01.01 AS Table 1, as spec
    /// §6.6 transcribes it. Written out in binary so a reader can compare
    /// with the table directly rather than trusting a hex conversion.
    #[test]
    fn the_apci_constants_are_application_layer_table_1() {
        assert_eq!(APCI_INDIVIDUAL_ADDRESS_WRITE, 0b00_1100_0000);
        assert_eq!(APCI_INDIVIDUAL_ADDRESS_READ, 0b01_0000_0000);
        assert_eq!(APCI_INDIVIDUAL_ADDRESS_RESPONSE, 0b01_0100_0000);
        // AL Figures 12–14, pp. 21–23.
        assert_eq!(APCI_IA_SERIAL_NUMBER_READ, 0b11_1101_1100);
        assert_eq!(APCI_IA_SERIAL_NUMBER_RESPONSE, 0b11_1101_1101);
        assert_eq!(APCI_IA_SERIAL_NUMBER_WRITE, 0b11_1101_1110);
        assert_eq!(APCI_MEMORY_READ, 0b10_0000_0000);
        assert_eq!(APCI_MEMORY_RESPONSE, 0b10_0100_0000);
        assert_eq!(APCI_MEMORY_WRITE, 0b10_1000_0000);
        assert_eq!(APCI_USER_MEMORY_READ, 0b10_1100_0000);
        assert_eq!(APCI_USER_MEMORY_RESPONSE, 0b10_1100_0001);
        assert_eq!(APCI_USER_MEMORY_WRITE, 0b10_1100_0010);
        assert_eq!(APCI_RESTART, 0b11_1000_0000);
        assert_eq!(APCI_AUTHORIZE_REQUEST, 0b11_1101_0001);
        assert_eq!(APCI_AUTHORIZE_RESPONSE, 0b11_1101_0010);
        assert_eq!(APCI_KEY_WRITE, 0b11_1101_0011);
        assert_eq!(APCI_PROPERTY_VALUE_READ, 0b11_1101_0101);
        assert_eq!(APCI_PROPERTY_VALUE_RESPONSE, 0b11_1101_0110);
        assert_eq!(APCI_PROPERTY_VALUE_WRITE, 0b11_1101_0111);
    }

    /// `A_Key_Write` has a constant and no variant, on purpose (spec
    /// §10.9). The constant exists so nobody reuses the value; the absence
    /// of a variant is what makes the service unsendable.
    #[test]
    fn key_write_has_an_apci_but_no_encoder() {
        let frame = mgmt(ApplicationService::Other {
            apci: APCI_KEY_WRITE,
            data: vec![0x00, 0x01, 0x02, 0x03, 0x04],
        });
        // It encodes as `Other` — the escape hatch, which a caller has to
        // ask for explicitly — and decodes back as `Other`, never as a
        // typed key write, because there is none.
        let decoded = decode_l_data(&encode_l_data(&frame).unwrap()).unwrap();
        assert!(matches!(
            decoded.service,
            ApplicationService::Other {
                apci: APCI_KEY_WRITE,
                ..
            }
        ));
    }

    /// `A_Memory_Read-PDU` (AL §3.4.4 Figure 74): `number` rides in the
    /// APCI's low six bits, then address high and low.
    #[test]
    fn memory_read_octets_match_figure_74() {
        let npdu = npdu_of(ApplicationService::MemoryRead {
            number: 3,
            address: 0x1234,
        });
        assert_eq!(
            npdu,
            vec![
                0x42, // T_Data_Connected seq 0 + APCI bits 9-8 = 10
                0x03, // APCI bits 7-6 = 00, number = 3
                0x12, 0x34,
            ]
        );
        assert_eq!(
            round_trip(ApplicationService::MemoryRead {
                number: 3,
                address: 0x1234
            }),
            ApplicationService::MemoryRead {
                number: 3,
                address: 0x1234
            }
        );
    }

    /// `number` is never a field next to the data it counts: it is
    /// `data.len()`, so a write claiming three octets and carrying four is
    /// not a shape this type can hold.
    #[test]
    fn memory_write_derives_its_number_from_the_data_it_carries() {
        let npdu = npdu_of(ApplicationService::MemoryWrite {
            address: 0x4000,
            data: vec![0xAA, 0xBB, 0xCC],
        });
        assert_eq!(npdu, vec![0x42, 0x83, 0x40, 0x00, 0xAA, 0xBB, 0xCC]);
        assert_eq!(
            round_trip(ApplicationService::MemoryWrite {
                address: 0x4000,
                data: vec![0xAA, 0xBB, 0xCC]
            }),
            ApplicationService::MemoryWrite {
                address: 0x4000,
                data: vec![0xAA, 0xBB, 0xCC]
            }
        );
    }

    /// The documented failure answer: *"the parameter number of the
    /// A_Memory_Response-PDU shall be zero and shall contain no data"*
    /// (spec §6.1). It has to be representable — a client that cannot
    /// receive a refusal cannot report one.
    #[test]
    fn a_memory_response_with_number_zero_is_the_failure_answer_and_not_an_error() {
        let npdu = npdu_of(ApplicationService::MemoryResponse {
            address: 0x4000,
            data: vec![],
        });
        assert_eq!(npdu, vec![0x42, 0x40, 0x40, 0x00]);
        assert_eq!(
            round_trip(ApplicationService::MemoryResponse {
                address: 0x4000,
                data: vec![]
            }),
            ApplicationService::MemoryResponse {
                address: 0x4000,
                data: vec![]
            }
        );
        // A write, by contrast, has nothing to say with zero octets.
        assert_eq!(
            encode_l_data(&mgmt(ApplicationService::MemoryWrite {
                address: 0x4000,
                data: vec![]
            }))
            .unwrap_err(),
            CemiError::MemoryOctetCountOutOfRange { number: 0, max: 63 }
        );
    }

    /// A frame whose `number` field disagrees with the octets that follow
    /// is not an `A_Memory_Response` — it is something this decoder cannot
    /// name, so it keeps every octet as `Other` rather than inventing a
    /// length (Global Constraint 2).
    #[test]
    fn a_memory_frame_whose_number_lies_stays_other_with_its_octets() {
        // APCI 0x244 claims four data octets; only one follows.
        let frame = mgmt(ApplicationService::Other {
            apci: 0x244,
            data: vec![0x40, 0x00, 0xAA],
        });
        let decoded = decode_l_data(&encode_l_data(&frame).unwrap()).unwrap();
        assert_eq!(
            decoded.service,
            ApplicationService::Other {
                apci: 0x244,
                data: vec![0x40, 0x00, 0xAA],
            }
        );
    }

    /// 64 octets do not fit a six-bit `number`. Rejected rather than
    /// truncated: a short write to the right address is worse than no
    /// write, because it looks like it worked.
    #[test]
    fn a_memory_write_of_sixty_four_octets_is_refused_not_truncated() {
        assert_eq!(
            encode_l_data(&mgmt(ApplicationService::MemoryWrite {
                address: 0x4000,
                data: vec![0u8; 64]
            }))
            .unwrap_err(),
            CemiError::MemoryOctetCountOutOfRange {
                number: 64,
                max: 63
            }
        );
        // 63 is the boundary and it fits.
        assert!(encode_l_data(&mgmt(ApplicationService::MemoryWrite {
            address: 0x4000,
            data: vec![0u8; 63]
        }))
        .is_ok());
    }

    /// `A_UserMemory_Write-PDU` (AL §3.5.6.3 Figure 81): a 20-bit address
    /// as *"4 bit address extension + 8 bit address high + 8 bit address
    /// low"*, with the four-bit `number` sharing the extension's octet.
    #[test]
    fn user_memory_octets_match_figure_81() {
        let npdu = npdu_of(ApplicationService::UserMemoryWrite {
            address: 0x7_1234,
            data: vec![0xDE, 0xAD],
        });
        assert_eq!(
            npdu,
            vec![
                0x42, // APCI bits 9-8 = 10
                0xC2, // APCI bits 7-0: 1100_0010 -> A_UserMemory_Write
                0x72, // address extension 7, number 2
                0x12, 0x34, 0xDE, 0xAD,
            ]
        );
        assert_eq!(
            round_trip(ApplicationService::UserMemoryWrite {
                address: 0x7_1234,
                data: vec![0xDE, 0xAD]
            }),
            ApplicationService::UserMemoryWrite {
                address: 0x7_1234,
                data: vec![0xDE, 0xAD]
            }
        );
        assert_eq!(
            round_trip(ApplicationService::UserMemoryRead {
                number: 15,
                address: 0xF_FFFF
            }),
            ApplicationService::UserMemoryRead {
                number: 15,
                address: 0xF_FFFF
            }
        );
    }

    /// The user-memory service's own limits, which are *not* the memory
    /// service's: 15 octets, not 63, and 20 address bits, not 16.
    #[test]
    fn user_memory_refuses_sixteen_octets_and_a_twenty_one_bit_address() {
        assert_eq!(
            encode_l_data(&mgmt(ApplicationService::UserMemoryWrite {
                address: 0x1_0000,
                data: vec![0u8; 16]
            }))
            .unwrap_err(),
            CemiError::MemoryOctetCountOutOfRange {
                number: 16,
                max: 15
            }
        );
        assert_eq!(
            encode_l_data(&mgmt(ApplicationService::UserMemoryWrite {
                address: USER_MEMORY_ADDRESS_LIMIT,
                data: vec![0xAA]
            }))
            .unwrap_err(),
            CemiError::UserMemoryAddressOutOfRange(USER_MEMORY_ADDRESS_LIMIT)
        );
    }

    /// `A_PropertyValue_Write-PDU`: object index, property id, then
    /// `nr_of_elem` (4 bits) and `start_index` (12 bits) packed into two
    /// octets — the exact frame spec §7.3's ten-octet load-state-control
    /// payload travels in.
    #[test]
    fn property_value_write_carries_the_ten_octet_load_control_payload() {
        let payload = vec![0x03, 0x0B, 0x00, 0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00];
        let npdu = npdu_of(ApplicationService::PropertyValueWrite {
            object_index: 2,
            property_id: 5,
            nr_of_elem: 1,
            start_index: 1,
            data: payload.clone(),
        });
        assert_eq!(npdu[0], 0x43); // APCI bits 9-8 = 11
        assert_eq!(npdu[1], 0xD7); // A_PropertyValue_Write
        assert_eq!(&npdu[2..6], &[0x02, 0x05, 0x10, 0x01]);
        assert_eq!(&npdu[6..], &payload[..]);
        assert_eq!(
            round_trip(ApplicationService::PropertyValueWrite {
                object_index: 2,
                property_id: 5,
                nr_of_elem: 1,
                start_index: 1,
                data: payload.clone(),
            }),
            ApplicationService::PropertyValueWrite {
                object_index: 2,
                property_id: 5,
                nr_of_elem: 1,
                start_index: 1,
                data: payload,
            }
        );
    }

    /// A property read refused by the device answers with `nr_of_elem = 0`
    /// and no data (spec §10.6), which must decode rather than be demoted.
    #[test]
    fn a_property_response_with_no_elements_is_the_refusal_shape() {
        assert_eq!(
            round_trip(ApplicationService::PropertyValueResponse {
                object_index: 0,
                property_id: 56,
                nr_of_elem: 0,
                start_index: 1,
                data: vec![],
            }),
            ApplicationService::PropertyValueResponse {
                object_index: 0,
                property_id: 56,
                nr_of_elem: 0,
                start_index: 1,
                data: vec![],
            }
        );
    }

    /// `nr_of_elem` and `start_index` share two octets, so an overflow in
    /// either silently rewrites the other. Refused on both.
    #[test]
    fn a_property_request_that_would_overflow_its_two_octets_is_refused() {
        assert_eq!(
            encode_l_data(&mgmt(ApplicationService::PropertyValueRead {
                object_index: 0,
                property_id: 5,
                nr_of_elem: 16,
                start_index: 1,
            }))
            .unwrap_err(),
            CemiError::PropertyRequestOutOfRange {
                nr_of_elem: 16,
                start_index: 1
            }
        );
        assert_eq!(
            encode_l_data(&mgmt(ApplicationService::PropertyValueRead {
                object_index: 0,
                property_id: 5,
                nr_of_elem: 1,
                start_index: 0x1000,
            }))
            .unwrap_err(),
            CemiError::PropertyRequestOutOfRange {
                nr_of_elem: 1,
                start_index: 0x1000
            }
        );
    }

    /// `A_Authorize_Request-PDU` (AL §3.5.7 Figure 86): one *"must be 0"*
    /// octet, then four key octets. The octet is written by the encoder,
    /// not by the caller, because there is nothing to decide about it.
    #[test]
    fn authorize_request_writes_the_reserved_octet_itself() {
        let npdu = npdu_of(ApplicationService::AuthorizeRequest {
            key: [0x12, 0x34, 0x56, 0x78],
        });
        assert_eq!(npdu, vec![0x43, 0xD1, 0x00, 0x12, 0x34, 0x56, 0x78]);
        assert_eq!(
            round_trip(ApplicationService::AuthorizeRequest {
                key: [0x12, 0x34, 0x56, 0x78]
            }),
            ApplicationService::AuthorizeRequest {
                key: [0x12, 0x34, 0x56, 0x78]
            }
        );
        // A request whose reserved octet is not 0 does not fit the PDU and
        // keeps its octets as `Other` rather than being read as a key.
        let odd = mgmt(ApplicationService::Other {
            apci: APCI_AUTHORIZE_REQUEST,
            data: vec![0x01, 0x12, 0x34, 0x56, 0x78],
        });
        assert!(matches!(
            decode_l_data(&encode_l_data(&odd).unwrap())
                .unwrap()
                .service,
            ApplicationService::Other { .. }
        ));
    }

    /// A bus monitor is the last place a device key should be readable
    /// (design spec §10.7), so the summary redacts it — while still naming the
    /// service, because a hidden authorisation attempt is worse.
    #[test]
    fn an_authorize_request_never_renders_its_key() {
        let service = ApplicationService::AuthorizeRequest {
            key: [0xDE, 0xAD, 0xBE, 0xEF],
        };
        let summary = service.payload_summary().unwrap();
        assert_eq!(service.variant_name(), "AuthorizeRequest");
        assert_eq!(summary, "key=<redacted>");
        assert!(!summary.contains("de"));
        assert!(!summary.contains("ad"));
    }

    /// The response is one level octet, where a *lower* number is more
    /// powerful (spec §10.1) — the codec carries it, the judging happens
    /// in `knx-core`.
    #[test]
    fn authorize_response_round_trips_every_level_octet() {
        for level in 0u8..=255 {
            assert_eq!(
                round_trip(ApplicationService::AuthorizeResponse { level }),
                ApplicationService::AuthorizeResponse { level }
            );
        }
    }

    /// `A_IndividualAddress_Response-PDU` has no data octets at all: the
    /// address is the frame's source (spec §4.2). This is the fieldless
    /// variant's whole point — there is nowhere to read a wrong address
    /// from.
    #[test]
    fn an_individual_address_response_names_its_sender_and_nothing_else() {
        let frame = LDataFrame {
            kind: LDataMessageKind::Indication,
            source: IndividualAddress::from_raw(0x1118),
            destination: Destination::Group(GroupAddress::from_raw(0x0000)),
            transport: Tpci::UnnumberedData,
            service: ApplicationService::IndividualAddressResponse,
        };
        let decoded = decode_l_data(&encode_l_data(&frame).unwrap()).unwrap();
        assert_eq!(
            decoded.service,
            ApplicationService::IndividualAddressResponse
        );
        assert_eq!(decoded.source, IndividualAddress::from_raw(0x1118));
    }

    /// `A_IndividualAddress_Write-PDU` carries the new address in two data
    /// octets, and the broadcast destination `0/0/0`.
    #[test]
    fn an_individual_address_write_carries_the_new_address() {
        let frame = LDataFrame {
            kind: LDataMessageKind::Request,
            source: IndividualAddress::from_raw(0x1101),
            destination: Destination::Group(GroupAddress::from_raw(0x0000)),
            transport: Tpci::UnnumberedData,
            service: ApplicationService::IndividualAddressWrite {
                address: IndividualAddress::from_raw(0x1118),
            },
        };
        let bytes = encode_l_data(&frame).unwrap();
        assert_eq!(&bytes[9..], &[0x00, 0xC0, 0x11, 0x18]);
        assert_eq!(
            decode_l_data(&bytes).unwrap().service,
            ApplicationService::IndividualAddressWrite {
                address: IndividualAddress::from_raw(0x1118),
            }
        );
    }

    /// The three serial-number PDUs, octet for octet against AL Figures
    /// 12–14 (pp. 21–23). The write's four reserved octets go out as zero;
    /// the response's individual address is the frame's source.
    #[test]
    fn the_serial_number_services_are_the_figures_octet_for_octet() {
        let sn = [0x00, 0x83, 0x12, 0x34, 0x56, 0x78];
        // Broadcast, connectionless: TPCI `00000000`, so the NPDU starts
        // `03` (the top APCI bits), not `43` as a numbered frame would.
        let frame = |service| LDataFrame {
            kind: LDataMessageKind::Request,
            source: IndividualAddress::from_raw(0x1101),
            destination: BROADCAST_DESTINATION,
            transport: Tpci::UnnumberedData,
            service,
        };
        let npdu_of = |service| encode_l_data(&frame(service)).unwrap()[9..].to_vec();
        let round_trip = |service| {
            decode_l_data(&encode_l_data(&frame(service)).unwrap())
                .unwrap()
                .service
        };
        let read = ApplicationService::IndividualAddressSerialNumberRead { serial_number: sn };
        assert_eq!(
            npdu_of(read.clone()),
            vec![0x03, 0xDC, 0x00, 0x83, 0x12, 0x34, 0x56, 0x78]
        );
        assert_eq!(
            round_trip(read),
            ApplicationService::IndividualAddressSerialNumberRead { serial_number: sn }
        );

        let write = ApplicationService::IndividualAddressSerialNumberWrite {
            serial_number: sn,
            address: IndividualAddress::from_raw(0x1144),
        };
        assert_eq!(
            npdu_of(write.clone()),
            vec![0x03, 0xDE, 0x00, 0x83, 0x12, 0x34, 0x56, 0x78, 0x11, 0x44, 0, 0, 0, 0]
        );
        assert_eq!(round_trip(write.clone()), write);

        let response = ApplicationService::IndividualAddressSerialNumberResponse {
            serial_number: sn,
            domain_address: 0x0000,
        };
        assert_eq!(
            npdu_of(response.clone()),
            vec![0x03, 0xDD, 0x00, 0x83, 0x12, 0x34, 0x56, 0x78, 0, 0, 0, 0]
        );
        assert_eq!(round_trip(response.clone()), response);

        // A PDU of the wrong length is not the service; it stays `Other`.
        let mut short = encode_l_data(&frame(write)).unwrap();
        short.truncate(short.len() - 1);
        short[8] -= 1;
        assert!(matches!(
            decode_l_data(&short).unwrap().service,
            ApplicationService::Other { apci: 0x3DE, .. }
        ));
        assert_eq!(format_serial_number(&sn), "0083:12345678");
    }

    /// `A_Restart`: type 0 is the unconfirmed Basic Restart, type 1 the
    /// confirmed Master Reset, and there is no type 2 (spec §8).
    #[test]
    fn restart_encodes_both_types_and_refuses_a_third() {
        assert_eq!(
            npdu_of(ApplicationService::Restart {
                response: false,
                restart_type: 0,
                data: vec![],
            }),
            vec![0x43, 0x80]
        );
        // A Master Reset request carries Erase Code and Channel Number.
        assert_eq!(
            npdu_of(ApplicationService::Restart {
                response: false,
                restart_type: 1,
                data: vec![0x01, 0x00],
            }),
            vec![0x43, 0x81, 0x01, 0x00]
        );
        // Its response sets bit 5.
        assert_eq!(
            npdu_of(ApplicationService::Restart {
                response: true,
                restart_type: 1,
                data: vec![0x00, 0x00, 0x05],
            }),
            vec![0x43, 0xA1, 0x00, 0x00, 0x05]
        );
        assert_eq!(
            encode_l_data(&mgmt(ApplicationService::Restart {
                response: false,
                restart_type: 2,
                data: vec![],
            }))
            .unwrap_err(),
            CemiError::InvalidRestartType(2)
        );
    }

    /// `A_Restart`'s bits 4-1 are reserved. A frame that sets them is not
    /// the restart service as this codec understands it, so it stays
    /// `Other` with its octets — the difference between "a device sent a
    /// restart" and "a device sent something restart-shaped" is worth
    /// keeping.
    #[test]
    fn a_restart_with_reserved_bits_set_is_not_read_as_a_restart() {
        let frame = mgmt(ApplicationService::Other {
            apci: APCI_RESTART | 0x02,
            data: vec![],
        });
        assert!(matches!(
            decode_l_data(&encode_l_data(&frame).unwrap())
                .unwrap()
                .service,
            ApplicationService::Other { .. }
        ));
    }

    /// Every typed management service survives encode → decode unchanged,
    /// and every one of them reports a name a log can print.
    #[test]
    fn every_management_service_round_trips_and_has_a_name() {
        let services = vec![
            ApplicationService::IndividualAddressRead,
            ApplicationService::IndividualAddressResponse,
            ApplicationService::IndividualAddressWrite {
                address: IndividualAddress::from_raw(0x1118),
            },
            ApplicationService::MemoryRead {
                number: 1,
                address: 0x0060,
            },
            ApplicationService::MemoryResponse {
                address: 0x0060,
                data: vec![0x81],
            },
            ApplicationService::MemoryWrite {
                address: 0x0060,
                data: vec![0x00],
            },
            ApplicationService::UserMemoryRead {
                number: 4,
                address: 0xA_0000,
            },
            ApplicationService::UserMemoryResponse {
                address: 0xA_0000,
                data: vec![1, 2, 3, 4],
            },
            ApplicationService::UserMemoryWrite {
                address: 0xA_0000,
                data: vec![1, 2, 3, 4],
            },
            ApplicationService::Restart {
                response: false,
                restart_type: 0,
                data: vec![],
            },
            ApplicationService::AuthorizeRequest { key: [1, 2, 3, 4] },
            ApplicationService::AuthorizeResponse { level: 3 },
            ApplicationService::PropertyValueRead {
                object_index: 0,
                property_id: 56,
                nr_of_elem: 1,
                start_index: 1,
            },
            ApplicationService::PropertyValueResponse {
                object_index: 0,
                property_id: 56,
                nr_of_elem: 1,
                start_index: 1,
                data: vec![0x00, 0x0F],
            },
            ApplicationService::PropertyValueWrite {
                object_index: 2,
                property_id: 5,
                nr_of_elem: 1,
                start_index: 1,
                data: vec![0x02; 10],
            },
        ];
        for service in services {
            assert_eq!(
                round_trip(service.clone()),
                service,
                "{} did not survive its own round trip",
                service.variant_name()
            );
            assert!(!service.variant_name().is_empty());
            // The two fieldless services have nothing to summarise, and
            // a summary invented for them would be noise in a monitor.
            let fieldless = matches!(
                service,
                ApplicationService::IndividualAddressRead
                    | ApplicationService::IndividualAddressResponse
            );
            assert_eq!(service.payload_summary().is_none(), fieldless);
        }
    }

    /// Nit 10: `Other { apci, .. }` above `0x3FF` does not fit the APCI's
    /// 10 bits — rejected on encode rather than masked (`apci: 0x07C0` used
    /// to silently encode and round-trip as `0x03C0`).
    #[test]
    fn other_apci_above_10_bits_is_rejected_not_masked() {
        let frame = LDataFrame {
            kind: LDataMessageKind::Request,
            source: IndividualAddress::from_raw(0x1101),
            destination: Destination::Individual(IndividualAddress::from_raw(0x1102)),
            transport: Tpci::UnnumberedData,
            service: ApplicationService::Other {
                apci: 0x07C0,
                data: vec![],
            },
        };
        assert_eq!(
            encode_l_data(&frame).unwrap_err(),
            CemiError::InvalidApci(0x07C0)
        );
    }

    /// §105: the four Transport Layer control TPDUs go out at `SYSTEM`
    /// priority, and `T_Connect`/`T_Disconnect` with `ack_request` set.
    /// TL v01.02.03 AS §3.7 (p. 13) and §3.8 (p. 14): "the priority shall
    /// be set to 'system'; the ack_request shall be set to true". §5.3
    /// (p. 19) A2/A3 (`T_ACK`), A4 (`T_NAK`) and A6 (`T_DISCONNECT`) say
    /// "priority = SYSTEM" and nothing about ack_request, so that bit stays
    /// clear on `T_ACK`/`T_NAK`. Ctrl1 layout: EMI_IMI v01.04.02 AS
    /// §4.1.5.3.2 (p. 76) `FT 0 R SB P P A C`; priority `00b` = system,
    /// `11b` = low (Data Link Layer General v01.03.02 AS §2.2.3).
    #[test]
    fn control_frames_request_system_priority_and_data_frames_stay_low() {
        let request = |transport, service| LDataFrame {
            kind: LDataMessageKind::Request,
            source: IndividualAddress::from_raw(0x0000),
            destination: Destination::Individual(IndividualAddress::from_raw(0x1143)),
            transport,
            service,
        };
        let ctrl1 = |frame: LDataFrame| encode_l_data(&frame).unwrap()[2];
        let none = || ApplicationService::NoApplicationPdu;
        // FT=1 R=1 SB=1, P=00 (system), A=1: 1011_0010.
        assert_eq!(ctrl1(request(Tpci::Connect, none())), 0xB2);
        assert_eq!(ctrl1(request(Tpci::Disconnect, none())), 0xB2);
        // P=00 (system), A=0: 1011_0000.
        assert_eq!(ctrl1(request(Tpci::Ack { seq: 3 }, none())), 0xB0);
        assert_eq!(ctrl1(request(Tpci::Nak { seq: 3 }, none())), 0xB0);
        // Data frames keep the crate's low-priority default: 1011_1100.
        let data = request(
            Tpci::NumberedData { seq: 0 },
            ApplicationService::DeviceDescriptorRead { descriptor_type: 0 },
        );
        assert_eq!(ctrl1(data), 0xBC);
        // An indication's priority is "don't care (11b)" (EMI §4.1.5.4.1):
        // the simulator's device-side control frames are not changed.
        let mut ind = request(Tpci::Ack { seq: 0 }, none());
        ind.kind = LDataMessageKind::Indication;
        assert_eq!(ctrl1(ind), 0xBC);
    }
}

/// K16: the domain-address services, the system broadcast and the RF medium
/// information, each against the figure it comes from.
#[cfg(test)]
mod domain_address_tests {
    use super::*;
    use knx_core::commissioning::domain_address::DomainAddress;

    const RF_DOA: [u8; 6] = [0x00, 0xFA, 0x12, 0x34, 0x56, 0x78];
    const SERIAL: [u8; 6] = [0x00, 0x83, 0x01, 0x02, 0x03, 0x04];

    fn frame(destination: Destination, service: ApplicationService) -> LDataFrame {
        LDataFrame {
            kind: LDataMessageKind::Request,
            source: IndividualAddress::from_raw(0x11FA),
            destination,
            transport: Tpci::UnnumberedData,
            service,
        }
    }

    fn round_trip(frame: &LDataFrame) -> LDataFrame {
        decode_l_data(&encode_l_data(frame).expect("encodes")).expect("decodes")
    }

    #[test]
    fn the_apcis_are_the_table_1_bits() {
        // AL §2.2 Table 1, p. 13: 1111100000/…01/…10 and 1111101100/…01/…10.
        assert_eq!(APCI_DOMAIN_ADDRESS_WRITE, 0b11_1110_0000);
        assert_eq!(APCI_DOMAIN_ADDRESS_READ, 0b11_1110_0001);
        assert_eq!(APCI_DOMAIN_ADDRESS_RESPONSE, 0b11_1110_0010);
        assert_eq!(APCI_DOA_SERIAL_NUMBER_READ, 0b11_1110_1100);
        assert_eq!(APCI_DOA_SERIAL_NUMBER_RESPONSE, 0b11_1110_1101);
        assert_eq!(APCI_DOA_SERIAL_NUMBER_WRITE, 0b11_1110_1110);
    }

    #[test]
    fn a_system_broadcast_clears_sb_and_a_broadcast_keeps_it() {
        let system = encode_l_data(&frame(
            SYSTEM_BROADCAST_DESTINATION,
            ApplicationService::DomainAddressRead,
        ))
        .unwrap();
        // Ctrl1 0xBC with bit 4 cleared; Ctrl2 group, hop count 6;
        // destination 0000h (DLL General §2.3).
        assert_eq!(system[2], 0xAC);
        assert_eq!(system[3], 0xE0);
        assert_eq!(&system[6..8], &[0x00, 0x00]);
        // Figure 22: TPCI octet with APCI bits 9-8, then 0xE1. L = 1.
        assert_eq!(&system[8..], &[0x01, 0x03, 0xE1]);

        let plain = encode_l_data(&frame(
            BROADCAST_DESTINATION,
            ApplicationService::IndividualAddressRead,
        ))
        .unwrap();
        assert_eq!(plain[2], 0xBC);
    }

    #[test]
    fn the_decoder_tells_the_two_broadcasts_apart() {
        let system = frame(
            SYSTEM_BROADCAST_DESTINATION,
            ApplicationService::DomainAddressRead,
        );
        assert_eq!(round_trip(&system), system);
        let plain = frame(
            BROADCAST_DESTINATION,
            ApplicationService::IndividualAddressRead,
        );
        assert_eq!(round_trip(&plain), plain);
        // SB clear on a group address that is not 0000h is not a system
        // broadcast: the flag is judged only where DLL §2.3 gives it meaning.
        let mut bytes = encode_l_data(&frame(
            Destination::Group(GroupAddress::from_raw(0x0903)),
            ApplicationService::GroupValueRead,
        ))
        .unwrap();
        bytes[2] &= !0x10;
        assert_eq!(
            decode_l_data(&bytes).unwrap().destination,
            Destination::Group(GroupAddress::from_raw(0x0903))
        );
    }

    #[test]
    fn the_domain_address_pdus_have_the_figures_shapes() {
        let write_rf = encode_l_data(&frame(
            SYSTEM_BROADCAST_DESTINATION,
            ApplicationService::DomainAddressWrite {
                domain_address: DomainAddress::Rf(RF_DOA),
            },
        ))
        .unwrap();
        // Figure 21: APCI, then six octets of domain address. L = 7.
        assert_eq!(write_rf[8], 7);
        assert_eq!(&write_rf[9..11], &[0x03, 0xE0]);
        assert_eq!(&write_rf[11..], &RF_DOA);

        let write_pl = encode_l_data(&frame(
            SYSTEM_BROADCAST_DESTINATION,
            ApplicationService::DomainAddressWrite {
                domain_address: DomainAddress::Powerline(0xBEEF),
            },
        ))
        .unwrap();
        // Figure 20: two octets, high first.
        assert_eq!(&write_pl[9..], &[0x03, 0xE0, 0xBE, 0xEF]);

        let sn_write = encode_l_data(&frame(
            SYSTEM_BROADCAST_DESTINATION,
            ApplicationService::DomainAddressSerialNumberWrite {
                serial_number: SERIAL,
                domain_address: DomainAddress::Rf(RF_DOA),
            },
        ))
        .unwrap();
        // Figure 30: serial number (octets 8-13), domain address (14-19).
        assert_eq!(&sn_write[9..11], &[0x03, 0xEE]);
        assert_eq!(&sn_write[11..17], &SERIAL);
        assert_eq!(&sn_write[17..], &RF_DOA);
    }

    #[test]
    fn every_domain_address_service_survives_its_round_trip() {
        for service in [
            ApplicationService::DomainAddressWrite {
                domain_address: DomainAddress::Rf(RF_DOA),
            },
            ApplicationService::DomainAddressWrite {
                domain_address: DomainAddress::Powerline(0x0102),
            },
            ApplicationService::DomainAddressRead,
            ApplicationService::DomainAddressResponse {
                domain_address: DomainAddress::Rf(RF_DOA),
            },
            ApplicationService::DomainAddressSerialNumberRead {
                serial_number: SERIAL,
            },
            ApplicationService::DomainAddressSerialNumberResponse {
                serial_number: SERIAL,
                domain_address: DomainAddress::Rf(RF_DOA),
            },
            ApplicationService::DomainAddressSerialNumberResponse {
                serial_number: SERIAL,
                domain_address: DomainAddress::Powerline(0x0102),
            },
            ApplicationService::DomainAddressSerialNumberWrite {
                serial_number: SERIAL,
                domain_address: DomainAddress::Rf(RF_DOA),
            },
        ] {
            let sent = frame(SYSTEM_BROADCAST_DESTINATION, service);
            assert_eq!(round_trip(&sent), sent);
        }
    }

    #[test]
    fn the_knx_ip_domain_address_forms_stay_undecoded_and_intact() {
        // Figure 31: a 4-octet multicast address. Not modelled, so `Other`
        // with every octet kept.
        let mut data = SERIAL.to_vec();
        data.extend_from_slice(&[224, 0, 23, 12]);
        let sent = frame(
            SYSTEM_BROADCAST_DESTINATION,
            ApplicationService::Other {
                apci: APCI_DOA_SERIAL_NUMBER_WRITE,
                data: data.clone(),
            },
        );
        assert_eq!(round_trip(&sent), sent);
        // A domain-address write of a length no medium uses is not one.
        let odd = frame(
            SYSTEM_BROADCAST_DESTINATION,
            ApplicationService::Other {
                apci: APCI_DOMAIN_ADDRESS_WRITE,
                data: vec![1, 2, 3],
            },
        );
        assert_eq!(round_trip(&odd), odd);
    }

    #[test]
    fn an_rf_frame_carries_its_medium_information_and_a_void_length() {
        let sent = frame(
            SYSTEM_BROADCAST_DESTINATION,
            ApplicationService::DomainAddressSerialNumberRead {
                serial_number: SERIAL,
            },
        );
        let rf = RfMediumInfo {
            info: 0x02, // battery ok, bidirectional
            serial_or_domain: [0; 6],
            lfn: None,
        };
        let bytes = encode_l_data_rf(&sent, &rf).unwrap();
        // EMI_IMI §4.1.4.3.2: AddIL 10, Type 02h, Len 08h, Info, SN (6),
        // LFN (255 = "insert your own").
        assert_eq!(&bytes[1..4], &[10, 0x02, 0x08]);
        assert_eq!(bytes[4], 0x02);
        assert_eq!(bytes[11], RF_LFN_VOID);
        // §4.1.5.4.1: L void.
        assert_eq!(bytes[12 + 6], 0x00);
        assert_eq!(decode_l_data(&bytes).unwrap(), sent);
        assert_eq!(
            decode_rf_medium_info(&bytes),
            Some(RfMediumInfo {
                lfn: Some(RF_LFN_VOID),
                ..rf
            })
        );
    }

    #[test]
    fn the_seven_octet_example_of_page_74_is_read_too() {
        // §4.1.4.3.10's example: `29h 9 02h 7 RF-Ctrl DoA6 … DoA1`, the
        // layout without LFN, then an RF L_Data.ind with L void.
        let mut bytes = vec![0x29, 9, 0x02, 7, 0x00];
        bytes.extend_from_slice(&RF_DOA);
        bytes.extend_from_slice(&[0xBC, 0xE0, 0x11, 0x05, 0x00, 0x00, 0x00, 0x03, 0xE1]);
        let decoded = decode_l_data(&bytes).unwrap();
        assert_eq!(decoded.service, ApplicationService::DomainAddressRead);
        assert_eq!(decoded.destination, BROADCAST_DESTINATION);
        let rf = decode_rf_medium_info(&bytes).unwrap();
        assert_eq!(rf.serial_or_domain, RF_DOA);
        assert_eq!(rf.lfn, None);
    }

    #[test]
    fn a_tp_frame_has_no_rf_medium_information() {
        let bytes = encode_l_data(&frame(
            BROADCAST_DESTINATION,
            ApplicationService::IndividualAddressRead,
        ))
        .unwrap();
        assert_eq!(decode_rf_medium_info(&bytes), None);
        // A TLV list that runs past its own end is not trusted.
        assert_eq!(decode_rf_medium_info(&[0x29, 3, 0x02, 8, 0x00]), None);
    }
}
