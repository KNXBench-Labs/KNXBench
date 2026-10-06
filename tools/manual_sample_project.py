#!/usr/bin/env python3
"""Build a fictional, schema-11 sample project for manual screenshots.

Every name, identifier and value is invented: the manufacturer "Example
Devices (fictional)" (M-7FF0) does not exist, no product is real. The project
is meant to show the KNXBench interface, not to prove ETS compatibility.
Usage: python3 tools/manual_sample_project.py OUT.knxproj (used by the
manual screenshot run, apps/knx-web/e2e/manual-screenshots.shots.ts).
"""
import sys
import zipfile

NS = 'http://knx.org/xml/project/11'
M = 'M-7FF0'


def xml(body):
    return f'<?xml version="1.0" encoding="utf-8"?>\n<KNX xmlns="{NS}" CreatedBy="KNXBench sample builder" ToolVersion="sample">{body}</KNX>'


DPTS = [  # (main, sub, name, text, size)
    (1, 1, 'DPT_Switch', 'switch', 1),
    (1, 8, 'DPT_UpDown', 'up/down', 1),
    (5, 1, 'DPT_Scaling', 'percentage (0..100%)', 8),
    (9, 1, 'DPT_Value_Temp', 'temperature (°C)', 16),
]


def master():
    dpts = ''.join(
        f'<DatapointType Id="DPT-{m}" Number="{m}" Name="{n.split("_")[0]}_{m}" Text="{m}.xxx" SizeInBit="{size}">'
        f'<DatapointSubtypes><DatapointSubtype Id="DPST-{m}-{s}" Number="{s}" Name="{n}" Text="{t}"/></DatapointSubtypes></DatapointType>'
        for m, s, n, t, size in DPTS)
    return xml('<MasterData Version="0" Signature="fictional">'
               f'<DatapointTypes>{dpts}</DatapointTypes>'
               '<MediumTypes><MediumType Id="MT-0" Number="0" Name="TP" Text="Twisted Pair"/></MediumTypes>'
               f'<Manufacturers><Manufacturer Id="{M}" Name="Example Devices (fictional)" KnxManufacturerId="32752"/></Manufacturers>'
               '</MasterData>')


# Programs: id suffix, name, channels, objects per channel, parameters
PROGRAMS = {
    'SA4': dict(app='A-0001-10-0001', name='Switch actuator 4-fold', number=1,
                channels=['Channel A', 'Channel B', 'Channel C', 'Channel D'],
                objects=[('Switch', 'On/Off', '1 Bit', 'DPST-1-1', True), ('Status', 'On/Off status', '1 Bit', 'DPST-1-1', False)]),
    'JA2': dict(app='A-0002-10-0001', name='Blind actuator 2-fold', number=2,
                channels=['Blind A', 'Blind B'],
                objects=[('Move', 'Up/Down', '1 Bit', 'DPST-1-8', True), ('Position', 'Position in %', '1 Byte', 'DPST-5-1', True)]),
    'PB4': dict(app='A-0003-10-0001', name='Push button 4-fold', number=3,
                channels=['Button 1', 'Button 2', 'Button 3', 'Button 4'],
                objects=[('Switch', 'On/Off', '1 Bit', 'DPST-1-1', True)]),
    'RT1': dict(app='A-0004-10-0001', name='Room thermostat', number=4,
                channels=['Heating'],
                objects=[('Actual temperature', 'Measured value', '2 Bytes', 'DPST-9-1', False),
                         ('Setpoint', 'Comfort setpoint', '2 Bytes', 'DPST-9-1', True),
                         ('Heating output', 'Valve position', '1 Byte', 'DPST-5-1', False)]),
}


def program_id(key):
    return f'{M}_{PROGRAMS[key]["app"]}'


def program(key):
    p = PROGRAMS[key]
    a = program_id(key)
    types = (f'<ParameterType Id="{a}_PT-Delay" Name="delay"><TypeNumber SizeInBit="8" Type="unsignedInt" minInclusive="0" maxInclusive="240"/></ParameterType>'
             f'<ParameterType Id="{a}_PT-Mode" Name="mode"><TypeRestriction Base="Value" SizeInBit="8">'
             f'<Enumeration Id="{a}_PT-Mode_EN-0" Text="Normally open" Value="0" DisplayOrder="0"/>'
             f'<Enumeration Id="{a}_PT-Mode_EN-1" Text="Normally closed" Value="1" DisplayOrder="1"/>'
             '</TypeRestriction></ParameterType>'
             f'<ParameterType Id="{a}_PT-Bus" Name="busreturn"><TypeRestriction Base="Value" SizeInBit="8">'
             f'<Enumeration Id="{a}_PT-Bus_EN-0" Text="Keep state" Value="0" DisplayOrder="0"/>'
             f'<Enumeration Id="{a}_PT-Bus_EN-1" Text="Off" Value="1" DisplayOrder="1"/>'
             f'<Enumeration Id="{a}_PT-Bus_EN-2" Text="On" Value="2" DisplayOrder="2"/>'
             '</TypeRestriction></ParameterType>')
    params, refs, dyn_channels = [], [], []
    params.append(f'<Parameter Id="{a}_P-1" Name="StartupDelay" Text="Delay after bus voltage recovery (s)" ParameterType="{a}_PT-Delay" Access="ReadWrite" Value="2"/>')
    refs.append(f'<ParameterRef Id="{a}_P-1_R-1" RefId="{a}_P-1" DisplayOrder="1" Tag="1"/>')
    general = f'<ChannelIndependentBlock><ParameterBlock Id="{a}_PB-1" Name="General" Text="General"><ParameterRefRef RefId="{a}_P-1_R-1"/></ParameterBlock></ChannelIndependentBlock>'
    objects, objrefs = [], []
    number = 0
    for c, channel in enumerate(p['channels'], start=1):
        pid_mode, pid_bus = 10 * c + 1, 10 * c + 2
        params.append(f'<Parameter Id="{a}_P-{pid_mode}" Name="Mode{c}" Text="Operating mode" ParameterType="{a}_PT-Mode" Access="ReadWrite" Value="0"/>')
        params.append(f'<Parameter Id="{a}_P-{pid_bus}" Name="BusReturn{c}" Text="Behaviour on bus voltage recovery" ParameterType="{a}_PT-Bus" Access="ReadWrite" Value="0"/>')
        refs.append(f'<ParameterRef Id="{a}_P-{pid_mode}_R-1" RefId="{a}_P-{pid_mode}" DisplayOrder="{10*c}" Tag="1"/>')
        refs.append(f'<ParameterRef Id="{a}_P-{pid_bus}_R-1" RefId="{a}_P-{pid_bus}" DisplayOrder="{10*c+1}" Tag="1"/>')
        orr = ''
        for name, function, size, dpt, write in p['objects']:
            oid = f'{a}_O-{number}'
            objects.append(f'<ComObject Id="{oid}" Name="{name}" Text="{channel}: {name}" Number="{number}" FunctionText="{function}" ObjectSize="{size}" '
                           f'ReadFlag="{"Disabled" if write else "Enabled"}" WriteFlag="{"Enabled" if write else "Disabled"}" CommunicationFlag="Enabled" '
                           f'TransmitFlag="{"Disabled" if write else "Enabled"}" UpdateFlag="Disabled" ReadOnInitFlag="Disabled" DatapointType="{dpt}"/>')
            objrefs.append(f'<ComObjectRef Id="{oid}_R-1" RefId="{oid}"/>')
            orr += f'<ComObjectRefRef RefId="{oid}_R-1"/>'
            number += 1
        dyn_channels.append(f'<Channel Id="{a}_CH-{c}" Name="Channel{c}" Text="{channel}" Number="{c}">'
                            f'<ParameterBlock Id="{a}_CH-{c}_PB-1" Name="Channel{c}Settings" Text="{channel}">'
                            f'<ParameterRefRef RefId="{a}_P-{pid_mode}_R-1"/><ParameterRefRef RefId="{a}_P-{pid_bus}_R-1"/>{orr}'
                            '</ParameterBlock></Channel>')
    body = (f'<ManufacturerData><Manufacturer RefId="{M}"><ApplicationPrograms>'
            f'<ApplicationProgram Id="{a}" Name="{p["name"]}" ApplicationNumber="{p["number"]}" ApplicationVersion="16" '
            f'ProgramType="ApplicationProgram" MaskVersion="MV-0701" DefaultLanguage="en-US" LoadProcedureStyle="ProductProcedure" PeiType="0">'
            f'<Static><ParameterTypes>{types}</ParameterTypes><Parameters>{"".join(params)}</Parameters>'
            f'<ParameterRefs>{"".join(refs)}</ParameterRefs><ComObjectTable>{"".join(objects)}</ComObjectTable>'
            f'<ComObjectRefs>{"".join(objrefs)}</ComObjectRefs></Static>'
            f'<Dynamic>{general}{"".join(dyn_channels)}</Dynamic>'
            '</ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData>')
    return xml(body), number


HARDWARE = [('SA4', 'EX-SA4', 'Switch actuator 4-fold, DIN rail'),
            ('JA2', 'EX-JA2', 'Blind actuator 2-fold, DIN rail'),
            ('PB4', 'EX-PB4', 'Push button 4-fold, flush mounted'),
            ('RT1', 'EX-RT1', 'Room thermostat with display')]


def hardware():
    items = ''
    for i, (key, order, text) in enumerate(HARDWARE, start=1):
        h = f'{M}_H-{order}-1'
        items += (f'<Hardware Id="{h}" Name="{order}" SerialNumber="{order}" VersionNumber="1" BusCurrent="10" HasIndividualAddress="1" HasApplicationProgram="1">'
                  f'<Products><Product Id="{h}_P-{order}" Text="{text}" OrderNumber="{order}" IsRailMounted="{1 if "DIN" in text else 0}" DefaultLanguage="en-US"/></Products>'
                  f'<Hardware2Programs><Hardware2Program Id="{h}_HP-{PROGRAMS[key]["app"][2:]}" MediumTypes="MT-0">'
                  f'<ApplicationProgramRef RefId="{program_id(key)}"/></Hardware2Program></Hardware2Programs></Hardware>')
    return xml(f'<ManufacturerData><Manufacturer RefId="{M}"><Hardware>{items}</Hardware></Manufacturer></ManufacturerData>')


def ref(key):
    order = dict((k, o) for k, o, _ in HARDWARE)[key]
    h = f'{M}_H-{order}-1'
    return f'{h}_P-{order}', f'{h}_HP-{PROGRAMS[key]["app"][2:]}'


def catalog():
    items = ''.join(
        f'<CatalogItem Id="{M}_CI-{i}" Name="{text}" Number="{i}" VisibleDescription="Fictional sample product" '
        f'ProductRefId="{ref(key)[0]}" Hardware2ProgramRefId="{ref(key)[1]}" DefaultLanguage="en-US"/>'
        for i, (key, _, text) in enumerate(HARDWARE, start=1))
    return xml(f'<ManufacturerData><Manufacturer RefId="{M}"><Catalog><CatalogSection Id="{M}_CS-1" Name="Sample devices" Number="1" DefaultLanguage="en-US">{items}</CatalogSection></Catalog></Manufacturer></ManufacturerData>')


GROUPS = [  # main, middle, name, [(sub, name)]
    (0, 0, ('Lighting', 'Switching'), [(1, 'Living room ceiling light'), (2, 'Kitchen light'), (3, 'Hall light'), (4, 'Bedroom light')]),
    (0, 1, ('Lighting', 'Status'), [(1, 'Living room ceiling light status'), (2, 'Kitchen light status'), (3, 'Hall light status'), (4, 'Bedroom light status')]),
    (1, 0, ('Blinds', 'Up/down'), [(1, 'Living room blind up/down'), (2, 'Bedroom blind up/down')]),
    (1, 1, ('Blinds', 'Position'), [(1, 'Living room blind position'), (2, 'Bedroom blind position')]),
    (2, 0, ('Heating', 'Temperatures'), [(1, 'Living room actual temperature'), (2, 'Living room setpoint')]),
    (2, 1, ('Heating', 'Valves'), [(1, 'Living room valve position')]),
]


def ga_id(main, middle, sub):
    return f'P-0001-0_GA-{main * 2048 + middle * 256 + sub}'


def project():
    # devices: (line, address, key, name, links per object number -> (main,middle,sub))
    devices = [
        (1, 1, 'SA4', 'Switch actuator ground floor', {0: (0, 0, 1), 1: (0, 1, 1), 2: (0, 0, 2), 3: (0, 1, 2), 4: (0, 0, 3), 5: (0, 1, 3)}),
        (1, 2, 'JA2', 'Blind actuator', {0: (1, 0, 1), 1: (1, 1, 1), 2: (1, 0, 2), 3: (1, 1, 2)}),
        (1, 10, 'PB4', 'Push button living room', {0: (0, 0, 1), 1: (0, 0, 2)}),
        (1, 11, 'PB4', 'Push button kitchen', {0: (0, 0, 2)}),
        (1, 20, 'RT1', 'Thermostat living room', {0: (2, 0, 1), 1: (2, 0, 2), 2: (2, 1, 1)}),
        (2, 1, 'SA4', 'Switch actuator first floor', {0: (0, 0, 4), 1: (0, 1, 4)}),
        (2, 10, 'PB4', 'Push button bedroom', {0: (0, 0, 4), 1: (0, 0, 3)}),
        (2, 11, 'PB4', 'Push button hall', {0: (0, 0, 3)}),
    ]
    lines = {1: '', 2: ''}
    places = {'Living room': [], 'Kitchen': [], 'Hall': [], 'Bedroom': [], 'Distribution board': []}
    where = {'Switch actuator ground floor': 'Distribution board', 'Blind actuator': 'Distribution board',
             'Push button living room': 'Living room', 'Push button kitchen': 'Kitchen', 'Thermostat living room': 'Living room',
             'Switch actuator first floor': 'Distribution board', 'Push button bedroom': 'Bedroom', 'Push button hall': 'Hall'}
    for i, (line, address, key, name, links) in enumerate(devices, start=1):
        a = program_id(key)
        product, h2p = ref(key)
        _, count = program(key)
        cors = ''
        for n in range(count):
            target = links.get(n)
            conn = ''
            if target:
                # Sensors send; an actuator receives on objects it may be
                # written through and sends its status objects.
                objects = PROGRAMS[key]['objects']
                writable = objects[n % len(objects)][4]
                tag = 'Send' if key == 'PB4' or not writable else 'Receive'
                conn = f'<Connectors><{tag} GroupAddressRefId="{ga_id(*target)}"/></Connectors>'
            cors += f'<ComObjectInstanceRef RefId="{a}_O-{n}_R-1" IsActive="1">{conn}</ComObjectInstanceRef>'
        params = f'<ParameterInstanceRefs><ParameterInstanceRef RefId="{a}_P-1_R-1" Value="{2 + i % 3}"/></ParameterInstanceRefs>'
        lines[line] += (f'<DeviceInstance Id="P-0001-0_DI-{i}" Name="{name}" Description="Fictional sample device" ProductRefId="{product}" '
                        f'Hardware2ProgramRefId="{h2p}" Address="{address}" CompletionStatus="Editing" '
                        'IndividualAddressLoaded="0" ApplicationProgramLoaded="0" ParametersLoaded="0" CommunicationPartLoaded="0" MediumConfigLoaded="0">'
                        f'{params}<ComObjectInstanceRefs>{cors}</ComObjectInstanceRefs></DeviceInstance>')
        places[where[name]].append(f'<DeviceInstanceRef RefId="P-0001-0_DI-{i}"/>')
    rooms_gf = ''.join(f'<BuildingPart Id="P-0001-0_BP-1{n}" Name="{r}" Type="Room">{"".join(places[r])}</BuildingPart>'
                       for n, r in enumerate(['Living room', 'Kitchen', 'Hall'], start=1))
    board = f'<BuildingPart Id="P-0001-0_BP-19" Name="Distribution board" Type="DistributionBoard">{"".join(places["Distribution board"])}</BuildingPart>'
    rooms_ff = f'<BuildingPart Id="P-0001-0_BP-21" Name="Bedroom" Type="Room">{"".join(places["Bedroom"])}</BuildingPart>'
    buildings = (f'<Buildings><BuildingPart Id="P-0001-0_BP-1" Name="Sample house" Type="Building">'
                 f'<BuildingPart Id="P-0001-0_BP-2" Name="Ground floor" Type="Floor">{rooms_gf}{board}</BuildingPart>'
                 f'<BuildingPart Id="P-0001-0_BP-3" Name="First floor" Type="Floor">{rooms_ff}</BuildingPart>'
                 '</BuildingPart></Buildings>')
    mains = {}
    for main, middle, (main_name, middle_name), addrs in GROUPS:
        mains.setdefault(main, (main_name, []))[1].append((middle, middle_name, addrs))
    ranges = ''
    for main, (main_name, middles) in sorted(mains.items()):
        inner = ''
        for middle, middle_name, addrs in middles:
            start = main * 2048 + middle * 256
            gas = ''.join(f'<GroupAddress Id="{ga_id(main, middle, sub)}" Address="{start + sub}" Name="{name}"/>' for sub, name in addrs)
            inner += f'<GroupRange Id="P-0001-0_GR-{main}{middle}" Name="{middle_name}" RangeStart="{start}" RangeEnd="{start + 255}">{gas}</GroupRange>'
        ranges += f'<GroupRange Id="P-0001-0_GR-{main}" Name="{main_name}" RangeStart="{main * 2048}" RangeEnd="{main * 2048 + 2047}">{inner}</GroupRange>'
    topo = (f'<Project Id="P-0001"><Installations><Installation InstallationId="0" Name="Sample house" DefaultLine="P-0001-0_L-1">'
            '<Topology><Area Id="P-0001-0_A-1" Name="House" Address="1">'
            f'<Line Id="P-0001-0_L-1" Name="Ground floor" Address="1" MediumTypeRefId="MT-0">{lines[1]}</Line>'
            f'<Line Id="P-0001-0_L-2" Name="First floor" Address="2" MediumTypeRefId="MT-0">{lines[2]}</Line>'
            f'</Area></Topology>{buildings}<GroupAddresses><GroupRanges>{ranges}</GroupRanges></GroupAddresses>'
            '</Installation></Installations></Project>')
    info = ('<Project Id="P-0001"><ProjectInformation Name="Sample house (fictional)" GroupAddressStyle="ThreeLevel" '
            'CompletionStatus="Editing"/></Project>')
    return xml(topo), xml(info)


def main():
    out = sys.argv[1]
    topo, info = project()
    entries = [('P-0001.signature', b'fictional sample, not signed'),
               ('P-0001/0.xml', topo.encode()), ('P-0001/Project.xml', info.encode()),
               ('knx_master.xml', master().encode()),
               (f'{M}/Hardware.xml', hardware().encode()), (f'{M}/Catalog.xml', catalog().encode())]
    for key in PROGRAMS:
        entries.append((f'{M}/{program_id(key)}.xml', program(key)[0].encode()))
    with zipfile.ZipFile(out, 'w', zipfile.ZIP_DEFLATED) as z:
        for name, data in entries:
            info_ = zipfile.ZipInfo(name, date_time=(2026, 10, 6, 0, 0, 0))
            info_.compress_type = zipfile.ZIP_DEFLATED
            z.writestr(info_, data)


if __name__ == '__main__':
    main()
