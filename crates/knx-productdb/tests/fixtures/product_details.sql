-- Seeds fictional, normalized product records for DB-to-HTTP-to-UI metadata regression tests.
INSERT INTO manufacturer(id,name) VALUES('M-0083','M-0083');
INSERT INTO source_file(sha256,source_path,len,bytes) VALUES('fixture-source','/home/alice/catalogue/fixture.xml',6,X'3C4B4E582F3E');
INSERT INTO hardware VALUES('H-rich','M-0083','Fictional hardware','SER-007','02','12.5',1,1,0,0,0,1,0,'M-0099','fixture-source');
INSERT INTO product VALUES('P-rich','M-0083','H-rich','Fictional actuator','ORDER-004',1,'72.5','en-US','product-hash','Registered','fixture-source');
INSERT INTO hardware2program VALUES('HP-rich','M-0083','H-rich','A-rich','MT-0 MT-5','link-hash','REG-1','Registered','signature-literal','fixture-source');
INSERT INTO application_program(id,manufacturer_id,name,application_number,application_version,program_type,mask_version,pei_type,load_procedure_style,default_language,hash,linkable,original_manufacturer,source_sha256,is_secure_enabled,max_security_group_key_table_entries,max_security_individual_address_entries,max_security_p2p_key_table_entries,max_tunneling_user_entries,max_user_entries,min_ets_version,replaces_versions)
VALUES('A-rich','M-0083','Fictional application','17','003','ApplicationProgram','MV-0701','0','ProductSpecific','en-US','program-hash',0,'M-0099','fixture-source','false','16','32','8','4','5','5.7','1 2');
INSERT INTO catalog_section VALUES('CS-root','M-0083',NULL,'Fictional catalogue','01','Root description','en-US','fixture-source');
INSERT INTO catalog_section VALUES('CS-child','M-0083','CS-root','Fictional section','02','Section description','en-US','fixture-source');
INSERT INTO catalog_item VALUES('CI-a','M-0083','CS-child','First placement','CAT-1','First description','P-rich','HP-rich','en-US','fixture-source');
INSERT INTO catalog_item VALUES('CI-b','M-0083','CS-root','Second placement','CAT-2','Second description','P-rich','HP-rich','en-US','fixture-source');
INSERT INTO package(sha256,source_name,scheme,size,bytes,unknown_count) VALUES('fixture-package','/home/alice/catalogue/fixture.knxprod',11,3,X'5A4950',1);
INSERT INTO package_member(package_sha256,ordinal,path,role,source_sha256,size) VALUES('fixture-package',1,'M-0083/fixture.xml','Hardware','fixture-source',6);
INSERT INTO package_source_name(package_sha256,source_name) VALUES('fixture-package','/home/alice/alias/vendor.zip!/inner/fixture.knxprod');
INSERT INTO ingest_unknown(source_sha256,program_id,xpath,kind,name,occurrences,sample) VALUES('fixture-source',NULL,'/KNX/ManufacturerData/Manufacturer/Hardware/Hardware','Attribute','FutureHardwareAttribute',2,'Preserved future value');
