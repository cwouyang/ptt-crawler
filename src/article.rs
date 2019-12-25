use std::net::Ipv4Addr;

use chrono::{prelude::*, DateTime};
use enum_from_str::ParseEnumVariantError;
use enum_from_str_derive::FromStr;

/// Article stores the parsed result of an article
#[derive(Deserialize, Clone, Debug)]
pub struct Article {
    pub id: String,
    pub category: String,
    pub title: String,
    pub author_id: String,
    pub author_name: Option<String>,
    pub board: BoardName,
    pub content: String,
    pub date: DateTime<FixedOffset>,
    pub ip: Ipv4Addr,
    pub reply_count: ReplyCount,
    pub replies: Vec<Reply>,
}

/// ReplyCount represents the number info about an article.
#[derive(Deserialize, Clone, Debug)]
pub struct ReplyCount {
    pub push: u16,
    pub neutral: u16,
    pub boo: u16,
}

/// Reply represents a reply.
#[derive(Deserialize, Clone, Debug)]
pub struct Reply {
    pub author_id: String,
    pub content: String,
    pub date: DateTime<FixedOffset>,
    pub reply_type: ReplyType,
}

/// ReplyType represents the type of a reply.
#[derive(Deserialize, Clone, Debug, FromStr, PartialEq)]
pub enum ReplyType {
    #[from_str = "推"]
    Push,
    #[from_str = "→"]
    Neutral,
    #[from_str = "噓"]
    Boo,
}

/// BoardName represents the name of a board.
/// Most of them are extracted from https://www.ptt.cc/bbs/hotboards.html
#[derive(Deserialize, Clone, Debug, FromStr, PartialEq)]
pub enum BoardName {
    AllTogether,
    #[from_str = "Bank_Service"]
    BankService,
    Baseball,
    #[from_str = "basketballTW"]
    BasketballTW,
    Beauty,
    BeautySalon,
    #[from_str = "biker"]
    Biker,
    #[from_str = "Boy-Girl"]
    BoyGirl,
    Brand,
    BuyTogether,
    #[from_str = "C_Chat"]
    CChat,
    #[from_str = "car"]
    Car,
    CarShop,
    #[from_str = "cat"]
    Cat,
    #[from_str = "China-Drama"]
    ChinaDrama,
    #[from_str = "cookclub"]
    CookClub,
    #[from_str = "creditcard"]
    CreditCard,
    #[from_str = "DC_SALE"]
    DcSale,
    #[from_str = "DMM_GAMES"]
    DmmGames,
    #[from_str = "Drama-Ticket"]
    DramaTicket,
    #[from_str = "DSLR"]
    Dslr,
    #[from_str = "E-appliance"]
    EAppliance,
    #[from_str = "e-shopping"]
    EShopping,
    Examination,
    #[from_str = "fastfood"]
    FastFood,
    #[from_str = "FATE_GO"]
    FateGo,
    Finance,
    Food,
    #[from_str = "forsale"]
    ForSale,
    #[from_str = "Gamesale"]
    GameSale,
    #[from_str = "gay"]
    Gay,
    #[from_str = "GBF"]
    Gbf,
    GetMarry,
    #[from_str = "give"]
    Give,
    Gossiping,
    HardwareSale,
    HatePolitics,
    #[from_str = "HBL"]
    Hbl,
    Headphone,
    Hearthstone,
    HelpBuy,
    #[from_str = "home-sale"]
    HomeSale,
    #[from_str = "hypermall"]
    HyperMall,
    Insurance,
    #[from_str = "iOS"]
    IOs,
    #[from_str = "IU"]
    Iu,
    #[from_str = "Japan_Travel"]
    JapanTravel,
    #[from_str = "japanavgirls"]
    JapanAvGirls,
    #[from_str = "Japandrama"]
    JapanDrama,
    #[from_str = "joke"]
    Joke,
    Kaohsiung,
    #[from_str = "Key_Mou_Pad"]
    KeyMouPad,
    KoreaDrama,
    KoreaStar,
    KoreanPop,
    Lakers,
    #[from_str = "lesbian"]
    Lesbian,
    #[from_str = "Lifeismoney"]
    LifeIsMoney,
    LoL,
    MacShop,
    MakeUp,
    #[from_str = "marriage"]
    Marriage,
    #[from_str = "marvel"]
    Marvel,
    MayDay,
    #[from_str = "medstudent"]
    Medstudent,
    Military,
    #[from_str = "MLB"]
    Mlb,
    MobileComm,
    #[from_str = "Mobile-game"]
    MobileGame,
    MobilePay,
    #[from_str = "mobilesales"]
    MobileSales,
    #[from_str = "movie"]
    Movie,
    MuscleBeach,
    #[from_str = "nb-shopping"]
    NbShopping,
    #[from_str = "NBA"]
    Nba,
    #[from_str = "NBA_Film"]
    NbaFilm,
    Nogizaka46,
    NSwitch,
    #[from_str = "ONE_PIECE"]
    OnePiece,
    #[from_str = "Palmar_Drama"]
    PalmarDrama,
    #[from_str = "part-time"]
    PartTime,
    #[from_str = "PathofExile"]
    PathOfExile,
    #[from_str = "PC_Shopping"]
    PcShopping,
    PlayStation,
    PokeMon,
    PokemonGO,
    PuzzleDragon,
    Salary,
    #[from_str = "sex"]
    Sex,
    #[from_str = "Soft_Job"]
    SoftJob,
    SportLottery,
    Steam,
    Stock,
    StupidClown,
    TaichungBun,
    Tainan,
    TaiwanDrama,
    #[from_str = "Tech_Job"]
    TechJob,
    ToS,
    #[from_str = "TW_Entertain"]
    TwEntertain,
    #[from_str = "TWICE"]
    Twice,
    TypeMoon,
    Wanted,
    #[from_str = "watch"]
    Watch,
    WomenTalk,
    #[from_str = "WOW"]
    Wow,
    Zastrology,
    #[from_str = "EAseries"]
    EASeries,
    Unknown,
}
