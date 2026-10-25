//! Each model's product render, from OpenRazer's daemon (GPL-2.0-or-later,
//! commit 477f4d6). The images belong to Razer and are fetched from its site.

pub(crate) static IMAGES: &[(u16, &str)] = &[
    (
        0x0015,
        "https://assets.razerzone.com/eeimages/products/40/razer-naga-molten-gallery-4.png",
    ),
    (0x001F, "https://hwimg.nl/Razer_naga-epic.png"),
    (
        0x0020,
        "https://assets.razerzone.com/eeimages/support/products/1277/1277_abyssus_2000.png",
    ),
    (
        0x0024,
        "https://assets.razerzone.com/eeimages/support/products/192/192_mamba_2012.png",
    ),
    (
        0x0025,
        "https://assets.razerzone.com/eeimages/support/products/192/192_mamba_2012.png",
    ),
    (
        0x002E,
        "https://assets.razerzone.com/eeimages/products/39/razer-naga-gallery-4.png",
    ),
    (
        0x002F,
        "https://assets.razerzone.com/eeimages/support/products/215/215_imperator.png",
    ),
    (
        0x0032,
        "https://assets.razerzone.com/eeimages/support/products/26/26_ouroboros.png",
    ),
    (
        0x0034,
        "https://assets.razerzone.com/eeimages/support/products/19/19_taipan.png",
    ),
    (
        0x0036,
        "https://assets.razerzone.com/eeimages/products/12/razer-naga-hex-gallery-12.png",
    ),
    (
        0x0037,
        "https://assets.razerzone.com/eeimages/support/products/561/561_deathadder_classic.png",
    ),
    (
        0x0038,
        "https://rzrwarranty.s3.amazonaws.com/a7daf40ad78c9584a693e310effa956019cdcd081391f93f71a7cd36d3dc577e.png",
    ),
    (
        0x0039,
        "https://assets.razerzone.com/eeimages/support/products/612/612_orochi_2015.png",
    ),
    (
        0x003E,
        "https://assets.razerzone.com/eeimages/products/20776/rzrnagaepicchroma_04.png",
    ),
    (
        0x003F,
        "https://assets.razerzone.com/eeimages/products/20776/rzrnagaepicchroma_04.png",
    ),
    (
        0x0040,
        "https://assets.razerzone.com/eeimages/support/products/227/227_razer_naga_2014.png",
    ),
    (
        0x0041,
        "https://assets.razerzone.com/eeimages/support/products/23/23_naga_hex.png",
    ),
    (
        0x0042,
        "https://assets.razerzone.com/eeimages/support/products/274/abyssus2014_500x500.png",
    ),
    (
        0x0043,
        "https://assets.razerzone.com/eeimages/support/products/278/278_deathadder_chroma.png",
    ),
    (
        0x0044,
        "https://assets.razerzone.com/eeimages/support/products/609/609_mamba_500x500.png",
    ),
    (
        0x0045,
        "https://assets.razerzone.com/eeimages/support/products/609/609_mamba_500x500.png",
    ),
    (
        0x0046,
        "https://assets.razerzone.com/eeimages/support/products/606/606_mambate_500x500.png",
    ),
    (
        0x0048,
        "https://assets.razerzone.com/eeimages/support/products/612/612_orochi_2015.png",
    ),
    (
        0x004C,
        "https://assets.razerzone.com/eeimages/support/products/613/613_diamondback.png",
    ),
    (
        0x004F,
        "https://assets2.razerzone.com/images/da10m/carousel/razer-death-adder-gallery-09.png",
    ),
    (
        0x0050,
        "https://assets.razerzone.com/eeimages/support/products/715/715_nagahexv2_500x500.png",
    ),
    (
        0x0053,
        "https://assets.razerzone.com/eeimages/support/products/636/636_naga_chroma.png",
    ),
    (
        0x0054,
        "https://assets.razerzone.com/eeimages/support/products/561/561_deathadder_classic.png",
    ),
    (
        0x0059,
        "https://assets.razerzone.com/eeimages/support/products/1205/1205_lancehead.png",
    ),
    (
        0x005A,
        "https://assets.razerzone.com/eeimages/support/products/1205/1205_lancehead.png",
    ),
    (
        0x005B,
        "https://assets.razerzone.com/eeimages/support/products/721/721_abyssusv2.png",
    ),
    (
        0x005C,
        "https://assets.razerzone.com/eeimages/support/products/724/724_deathadderelite_500x500.png",
    ),
    (
        0x005E,
        "https://assets.razerzone.com/eeimages/support/products/1277/1277_abyssus_2000.png",
    ),
    (
        0x0060,
        "https://assets.razerzone.com/eeimages/support/products/1203/1206_lanceheadte.png",
    ),
    (
        0x0062,
        "https://assets.razerzone.com/eeimages/support/products/1234/1234_atheris.png",
    ),
    (
        0x0064,
        "https://assets.razerzone.com/eeimages/support/products/1241/1241_basilisk.png",
    ),
    (
        0x0065,
        "https://hybrismediaprod.blob.core.windows.net/sys-master-phoenix-images-container/h19/h61/9080617500702/Basilisk-Essential-500x500.png",
    ),
    (
        0x0067,
        "https://assets.razerzone.com/eeimages/support/products/1251/1251_razer_naga_trinity.png",
    ),
    (
        0x006A,
        "https://assets.razerzone.com/eeimages/support/products/1288/d.va_abyssus_elite.png",
    ),
    (
        0x006B,
        "https://assets.razerzone.com/eeimages/support/products/1290/1290_abyssusessential.png",
    ),
    (
        0x006C,
        "https://assets.razerzone.com/eeimages/support/products/1390/1390_mamba_elite.png",
    ),
    (
        0x006E,
        "https://assets.razerzone.com/eeimages/support/products/1385/1385_deathadderessential.png",
    ),
    (
        0x006F,
        "https://assets.razerzone.com/eeimages/support/products/1205/1205_lancehead.png",
    ),
    (
        0x0070,
        "https://assets.razerzone.com/eeimages/support/products/1205/1205_lancehead.png",
    ),
    (
        0x0071,
        "https://assets2.razerzone.com/images/da10m/carousel/razer-death-adder-gallery-25.png",
    ),
    (
        0x0072,
        "https://assets.razerzone.com/eeimages/support/products/1404/1404_mamba_wireless.png",
    ),
    (
        0x0073,
        "https://assets.razerzone.com/eeimages/support/products/1404/1404_mamba_wireless.png",
    ),
    (0x0077, "https://dl.razerzone.com/src/3802-0-en-v1.png"),
    (
        0x0078,
        "https://assets.razerzone.com/eeimages/support/products/1539/1539_viper.png",
    ),
    (
        0x007A,
        "https://assets.razerzone.com/eeimages/support/products/1577/ee_photo.png",
    ),
    (
        0x007B,
        "https://assets.razerzone.com/eeimages/support/products/1577/ee_photo.png",
    ),
    (
        0x007C,
        "https://assets.razerzone.com/eeimages/support/products/1714/comp_1_00000.png",
    ),
    (
        0x007D,
        "https://assets.razerzone.com/eeimages/support/products/1714/comp_1_00000.png",
    ),
    (0x0080, "https://dl.razerzone.com/src/3802-0-en-v1.png"),
    (
        0x0083,
        "https://assets.razerzone.com/eeimages/support/products/1589/1589_basilisk_x__hyperspeed.png",
    ),
    (
        0x0084,
        "https://assets.razerzone.com/eeimages/support/products/1612/1612_razerdeathadderv2.png",
    ),
    (
        0x0085,
        "https://assets.razerzone.com/eeimages/support/products/1617/1617_basilisk-v2.png",
    ),
    (
        0x0086,
        "https://assets.razerzone.com/eeimages/support/products/1590/1590_basilisk_ultimate.png",
    ),
    (
        0x0088,
        "https://assets.razerzone.com/eeimages/support/products/1590/1590_basilisk_ultimate.png",
    ),
    (
        0x008A,
        "https://assets.razerzone.com/eeimages/support/products/1634/vipermini.png",
    ),
    (
        0x008C,
        "https://assets.razerzone.com/eeimages/support/products/1692/deathadder-v2-mini.png",
    ),
    (
        0x008D,
        "https://rzrwarranty.s3.amazonaws.com/cee694cd7526df413008167b7566af310985321b20c57f3dc42e5cbd773f2417.png",
    ),
    (
        0x008F,
        "https://hybrismediaprod.blob.core.windows.net/sys-master-phoenix-images-container/hfd/ha6/9080569528350/razer-naga-pro-500x500.png",
    ),
    (
        0x0090,
        "https://hybrismediaprod.blob.core.windows.net/sys-master-phoenix-images-container/hfd/ha6/9080569528350/razer-naga-pro-500x500.png",
    ),
    (0x0091, "https://dl.razerzone.com/Images/Viper%208KHz/Viper8khz.png"),
    (0x0094, "https://dl.razerzone.com/src/OrochiV2-1-en-v1.png"),
    (0x0095, "https://dl.razerzone.com/src/OrochiV2-1-en-v1.png"),
    (0x0096, "https://dl.razerzone.com/src/3993-1-EN-V2.png"),
    (
        0x0098,
        "https://assets.razerzone.com/eeimages/support/products/1385/1385_deathadderessential.png",
    ),
    (
        0x0099,
        "https://assets2.razerzone.com/pages/basilisk-v3-4D578898E8144Le8da21dde32b7a9f5f/basilisk.png",
    ),
    (0x009A, "https://dl.razerzone.com/src/5763/5763-1-en-v1.png"),
    (
        0x009C,
        "https://hybrismediaprod.blob.core.windows.net/sys-master-phoenix-images-container/he8/h51/9250345058334/deathadder-v2-x-hyperspeed-500x500.png",
    ),
    (0x009E, "https://dl.razerzone.com/src2/9682/9682-1-en-v1.png"),
    (0x009F, "https://dl.razerzone.com/src2/9682/9682-1-en-v1.png"),
    (
        0x00A1,
        "https://assets.razerzone.com/eeimages/support/products/1692/deathadder-v2-mini.png",
    ),
    (
        0x00A3,
        "https://hybrismediaprod.blob.core.windows.net/sys-master-phoenix-images-container/h54/h60/9591466950686/cobra-500x500.png",
    ),
    (0x00A5, "https://dl.razerzone.com/src/6048-1-en-v10.png"),
    (0x00A6, "https://dl.razerzone.com/src/6048-1-en-v10.png"),
    (0x00A7, "https://dl.razerzone.com/src2/6292/6292-1-en-v1.png"),
    (0x00A8, "https://dl.razerzone.com/src2/6292/6292-1-en-v1.png"),
    (0x00AA, "https://dl.razerzone.com/src2/6220/6220-4-en-v1.png"),
    (0x00AB, "https://dl.razerzone.com/src2/6220/6220-4-en-v1.png"),
    (0x00AF, "https://dl.razerzone.com/src2/13182/13182-1-en-v2.png"),
    (0x00B0, "https://dl.razerzone.com/src2/13182/13182-1-en-v2.png"),
    (0x00B2, "https://dl.razerzone.com/src/6124/6124-1-en-v2.png"),
    (0x00B3, "https://dl.razerzone.com/src/6141/6141-1-en-v1.png"),
    (
        0x00B4,
        "https://hybrismediaprod.blob.core.windows.net/sys-master-phoenix-images-container%2Fh4c%2Fh44%2F9451887460382%2Fnaga-v2-hyperspeed-500x500.png",
    ),
    (0x00B6, "https://dl.razerzone.com/src/6130/6130-1-en-v2.png"),
    (0x00B7, "https://dl.razerzone.com/src/6130/6130-1-en-v2.png"),
    (0x00B8, "https://dl.razerzone.com/src2/13432/13432-1-en-v3.png"),
    (0x00B9, "https://dl.razerzone.com/src2/9766/9766-1-en-v1.png"),
    (
        0x00BE,
        "https://medias-p1.phoenix.razer.com/sys-master-phoenix-images-container/h01/hf3/9926511951902/deathadder-v4-pro-black-500x500.png",
    ),
    (
        0x00BF,
        "https://medias-p1.phoenix.razer.com/sys-master-phoenix-images-container/h01/hf3/9926511951902/deathadder-v4-pro-black-500x500.png",
    ),
    (0x00C0, "https://dl.razerzone.com/src2/14044/14044-1-en-v1.png"),
    (0x00C1, "https://dl.razerzone.com/src2/14044/14044-1-en-v1.png"),
    (0x00C2, "https://dl.razerzone.com/src/6130/6130-1-en-v2.png"),
    (0x00C3, "https://dl.razerzone.com/src/6130/6130-1-en-v2.png"),
    (0x00C4, "https://dl.razerzone.com/src/6124/6124-1-en-v2.png"),
    (0x00C5, "https://dl.razerzone.com/src/6124/6124-1-en-v2.png"),
    (0x00C7, "https://dl.razerzone.com/src2/15010/15010-1-en-v1.png"),
    (0x00C8, "https://dl.razerzone.com/src2/15010/15010-1-en-v1.png"),
    (
        0x00CB,
        "https://hybrismediaprod.blob.core.windows.net/sys-master-phoenix-images-container/he4/h63/9822107926558/basilisk-v3-35k-2-500x500.png",
    ),
    (0x00CC, "https://dl.razerzone.com/src2/14676/14676-1-en-v1.png"),
    (0x00CD, "https://dl.razerzone.com/src2/14676/14676-1-en-v1.png"),
    (
        0x00D0,
        "https://medias-p1.phoenix.razer.com/sys-master-phoenix-images-container/ha3/h76/9899953717278/pro-click-v2-black-500x500.png",
    ),
    (
        0x00D1,
        "https://medias-p1.phoenix.razer.com/sys-master-phoenix-images-container/ha3/h76/9899953717278/pro-click-v2-black-500x500.png",
    ),
    (0x00D3, "https://dl.razerzone.com/src2/15485/15485-1-en-v2.png"),
    (0x00D4, "https://dl.razerzone.com/src2/15485/15485-1-en-v2.png"),
    // The Phantom Green Edition has only a JPEG; its shape is the 35K's.
    (0x00D6, "https://dl.razerzone.com/src2/14676/14676-1-en-v1.png"),
    (0x00D7, "https://dl.razerzone.com/src2/14676/14676-1-en-v1.png"),
    (0x00DA, "https://dl.razerzone.com/src2/15141/15141-1-en-v1.png"),
    (0x00DB, "https://dl.razerzone.com/src2/15141/15141-1-en-v1.png"),
    (0x00DE, "https://dl.razerzone.com/src2/14044/14044-1-en-v1.png"),
    (0x00DF, "https://dl.razerzone.com/src2/14044/14044-1-en-v1.png"),
    // From OpenRazer pull request 2751.
    (0x00E5, "https://dl.razerzone.com/src2/20779/20779-01-en-v1.png"),
    (0x00E6, "https://dl.razerzone.com/src2/20779/20779-01-en-v1.png"),
];
