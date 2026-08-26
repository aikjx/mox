use mox_cloud_sdk::prelude::*;

#[tokio::main]
async fn main() {
    // Known CRC-64/ECMA-182 vector for ASCII "123456789"
    let known: u64 = 0x6C40DF5F0B497347;
    let computed = crc64_ecma(0, b"123456789");
    assert_eq!(
        computed, known,
        "known vector mismatch: computed={:#x} expected={:#x}",
        computed, known
    );

    // Incremental CRC across two parts equals CRC of concatenated bytes
    let p1 = vec![0xAAu8; 512];
    let p2 = vec![0x55u8; 512];
    let combined: Vec<u8> = p1.iter().chain(p2.iter()).copied().collect();
    let direct = crc64_ecma(0, &combined);
    let step = crc64_ecma(crc64_ecma(0, &p1), &p2);
    assert_eq!(direct, step, "incremental CRC aggregation must match");

    // Verify with SDK client upload
    let client = Client::new();
    let uid = client
        .create_multipart_upload("crcb", "crc/checked.bin")
        .await
        .unwrap();
    let _pe1 = client
        .upload_part("crcb", "crc/checked.bin", &uid, 1, p1.clone())
        .await
        .unwrap();
    let _pe2 = client
        .upload_part("crcb", "crc/checked.bin", &uid, 2, p2.clone())
        .await
        .unwrap();
    let pe_all = vec![_pe1, _pe2];
    let _etag = client
        .complete_multipart_upload("crcb", "crc/checked.bin", &uid, pe_all)
        .await
        .unwrap();
    let obj = client.get_object("crcb", "crc/checked.bin").await.unwrap();
    let obj_crc = crc64_ecma(0, &obj);
    assert_eq!(obj_crc, direct, "final assembled object CRC must match direct CRC");

    println!(
        "XJ-OK: t3_06_crc_check known_vec={:#x} incremental={:#x} obj_crc={:#x}",
        computed, step, obj_crc
    );
}
