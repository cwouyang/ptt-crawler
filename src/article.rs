use std::net::Ipv4Addr;

use chrono::{prelude::*, DateTime};

/// Meta stores the parsed result of an article meta.
#[derive(Deserialize, Clone, Debug)]
pub struct Meta {
    pub board: BoardName,
    pub id: String,
    pub category: String,
    pub title: String,
    pub author_id: String,
    pub author_name: Option<String>,
    pub date: Option<DateTime<FixedOffset>>,
    pub ip: Option<Ipv4Addr>,
}

/// Article stores the parsed result of an article.
#[derive(Deserialize, Clone, Debug)]
pub struct Article {
    pub meta: Meta,
    pub content: String,
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
    pub reply_type: ReplyType,
    pub author_id: String,
    pub ip: Option<Ipv4Addr>,
    pub date: Option<DateTime<FixedOffset>>,
    pub content: String,
}

/// ReplyType represents the type of a reply.
#[derive(Deserialize, Clone, Debug, EnumString, PartialEq)]
pub enum ReplyType {
    #[strum(serialize = "推")]
    Push,
    #[strum(serialize = "→")]
    Neutral,
    #[strum(serialize = "噓")]
    Boo,
}

/// BoardName represents the name of a board.
/// Most of them are extracted from https://www.ptt.cc/bbs/hotboards.html
#[derive(Deserialize, Clone, Debug, EnumString, Display, PartialEq)]
pub enum BoardName {
    AllTogether,
    #[strum(serialize = "Bank_Service")]
    BankService,
    Baseball,
    #[strum(serialize = "basketballTW")]
    BasketballTW,
    Beauty,
    BeautySalon,
    #[strum(serialize"biker")]
    Biker,
    #[strum(serialize = "Boy-Girl")]
    BoyGirl,
    Brand,
    BuyTogether,
    #[strum(serialize = "C_Chat")]
    CChat,
    #[strum(serialize = "car")]
    Car,
    CarShop,
    #[strum(serialize = "cat")]
    Cat,
    #[strum(serialize = "China-Drama")]
    ChinaDrama,
    #[strum(serialize = "cookclub")]
    CookClub,
    #[strum(serialize = "creditcard")]
    CreditCard,
    #[strum(serialize = "DC_SALE")]
    DcSale,
    #[strum(serialize = "DMM_GAMES")]
    DmmGames,
    #[strum(serialize = "Drama-Ticket")]
    DramaTicket,
    #[strum(serialize = "DSLR")]
    Dslr,
    #[strum(serialize = "E-appliance")]
    EAppliance,
    #[strum(serialize = "e-shopping")]
    EShopping,
    Examination,
    #[strum(serialize = "fastfood")]
    FastFood,
    #[strum(serialize = "FATE_GO")]
    FateGo,
    Finance,
    Food,
    #[strum(serialize = "forsale")]
    ForSale,
    #[strum(serialize = "Gamesale")]
    GameSale,
    #[strum(serialize = "gay")]
    Gay,
    #[strum(serialize = "GBF")]
    Gbf,
    GetMarry,
    #[strum(serialize = "give")]
    Give,
    Gossiping,
    HardwareSale,
    HatePolitics,
    #[strum(serialize = "HBL")]
    Hbl,
    Headphone,
    Hearthstone,
    HelpBuy,
    #[strum(serialize = "home-sale")]
    HomeSale,
    #[strum(serialize = "hypermall")]
    HyperMall,
    Insurance,
    #[strum(serialize = "iOS")]
    IOs,
    #[strum(serialize = "IU")]
    Iu,
    #[strum(serialize = "Japan_Travel")]
    JapanTravel,
    #[strum(serialize = "japanavgirls")]
    JapanAvGirls,
    #[strum(serialize = "Japandrama")]
    JapanDrama,
    #[strum(serialize = "joke")]
    Joke,
    Kaohsiung,
    #[strum(serialize = "Key_Mou_Pad")]
    KeyMouPad,
    KoreaDrama,
    KoreaStar,
    KoreanPop,
    Lakers,
    #[strum(serialize = "lesbian")]
    Lesbian,
    #[strum(serialize = "Lifeismoney")]
    LifeIsMoney,
    LoL,
    MacShop,
    MakeUp,
    #[strum(serialize = "marriage")]
    Marriage,
    #[strum(serialize = "marvel")]
    Marvel,
    MayDay,
    #[strum(serialize = "medstudent")]
    Medstudent,
    Military,
    #[strum(serialize = "MLB")]
    Mlb,
    MobileComm,
    #[strum(serialize = "Mobile-game")]
    MobileGame,
    MobilePay,
    #[strum(serialize = "mobilesales")]
    MobileSales,
    #[strum(serialize = "movie")]
    Movie,
    MuscleBeach,
    #[strum(serialize = "nb-shopping")]
    NbShopping,
    #[strum(serialize = "NBA")]
    Nba,
    #[strum(serialize = "NBA_Film")]
    NbaFilm,
    Nogizaka46,
    NSwitch,
    #[strum(serialize = "ONE_PIECE")]
    OnePiece,
    #[strum(serialize = "Palmar_Drama")]
    PalmarDrama,
    #[strum(serialize = "part-time")]
    PartTime,
    #[strum(serialize = "PathofExile")]
    PathOfExile,
    #[strum(serialize = "PC_Shopping")]
    PcShopping,
    PlayStation,
    PokeMon,
    PokemonGO,
    PuzzleDragon,
    Salary,
    #[strum(serialize = "sex")]
    Sex,
    #[strum(serialize = "Soft_Job")]
    SoftJob,
    SportLottery,
    Steam,
    Stock,
    StupidClown,
    TaichungBun,
    Tainan,
    TaiwanDrama,
    #[strum(serialize = "Tech_Job")]
    TechJob,
    ToS,
    #[strum(serialize = "TW_Entertain")]
    TwEntertain,
    #[strum(serialize = "TWICE")]
    Twice,
    TypeMoon,
    Wanted,
    #[strum(serialize = "watch")]
    Watch,
    WomenTalk,
    #[strum(serialize = "WOW")]
    Wow,
    Zastrology,
    #[strum(serialize = "EAseries")]
    EASeries,
    Unknown,
}
