// use std::collections::HashMap;

// #[derive(Clone, PartialEq, Debug)]
// #[repr(u8)]
// pub enum Style {
//     Rock,
//     Country,
//     Blues,
//     Metal,
//     Funk,
//     Jazz,
//     Ballad,
//     Pop,
//     Reggae,
//     Electronic,
// }

// #[derive(Clone, PartialEq, Debug)]
// #[repr(u8)]
// pub enum TimeSignature {
//     TwoFour = 0x24,
//     FourFour = 0x44,
//     TwelveFour = 0xc4,
//     ThreeFour = 0x34,
//     SixEight = 0x68,
// }

// pub const DRUMS: HashMap<&'static str, (Style, TimeSignature)> = HashMap::from([
//     ("Standard", (Style::Rock, TimeSignature::TwoFour)),
//     ("Swing rock", (Style::Rock, TimeSignature::TwelveFour)),
//     ("Power beat", (Style::Rock, TimeSignature::FourFour)),
//     ("Smooth", (Style::Rock, TimeSignature::FourFour)),
//     ("Mega drive", (Style::Rock, TimeSignature::FourFour)),
//     ("Hard rock", (Style::Rock, TimeSignature::FourFour)),
//     ("Boogie", (Style::Rock, TimeSignature::FourFour)),
//     ("Walk line", (Style::Country, TimeSignature::FourFour)),
//     ("Blue grass", (Style::Country, TimeSignature::FourFour)),
//     ("Waltz", (Style::Country, TimeSignature::ThreeFour)),
//     ("Train", (Style::Country, TimeSignature::FourFour)),
//     ("Country rock", (Style::Country, TimeSignature::FourFour)),
//     ("Slowly", (Style::Country, TimeSignature::FourFour)),
//     ("Slow blues", (Style::Blues, TimeSignature::TwelveFour)),
//     ("Chicago", (Style::Blues, TimeSignature::FourFour)),
//     ("R&B", (Style::Blues, TimeSignature::FourFour)),
//     ("Blues rock", (Style::Blues, TimeSignature::FourFour)),
//     ("Road train", (Style::Blues, TimeSignature::TwelveFour)),
//     ("Shuffle", (Style::Blues, TimeSignature::FourFour)),
//     ("2x bass", (Style::Metal, TimeSignature::FourFour)),
//     ("Close beat", (Style::Metal, TimeSignature::FourFour)),
//     ("Heavy bass", (Style::Metal, TimeSignature::FourFour)),
//     ("Fast", (Style::Metal, TimeSignature::FourFour)),
//     ("Holy case", (Style::Metal, TimeSignature::FourFour)),
//     ("Open hat", (Style::Metal, TimeSignature::FourFour)),
//     ("Epic", (Style::Metal, TimeSignature::FourFour)),
//     ("Bouns", (Style::Funk, TimeSignature::FourFour)),
//     ("East coast", (Style::Funk, TimeSignature::FourFour)),
//     ("New mann", (Style::Funk, TimeSignature::FourFour)),
//     ("R&B funk", (Style::Funk, TimeSignature::FourFour)),
//     ("Soul", (Style::Funk, TimeSignature::FourFour)),
//     ("Uncle jam", (Style::Funk, TimeSignature::FourFour)),
//     ("80 func", (Style::Funk, TimeSignature::FourFour)),
//     ("Bluesy", (Style::Ballad, TimeSignature::SixEight)),
//     ("Grooves", (Style::Ballad, TimeSignature::FourFour)),
//     ("Ballad rock", (Style::Ballad, TimeSignature::FourFour)),
//     ("Slow rock", (Style::Ballad, TimeSignature::FourFour)),
//     ("Tutorial", (Style::Ballad, TimeSignature::FourFour)),
//     ("R&B ballad", (Style::Ballad, TimeSignature::FourFour)),
//     ("Gospel", (Style::Ballad, TimeSignature::FourFour)),
//     ("Beach side", (Style::Pop, TimeSignature::TwoFour)),
//     ("Big city", (Style::Pop, TimeSignature::FourFour)),
//     ("Funky pop", (Style::Pop, TimeSignature::FourFour)),
//     ("Modern", (Style::Pop, TimeSignature::FourFour)),
//     ("School pop", (Style::Pop, TimeSignature::FourFour)),
//     ("Motown", (Style::Pop, TimeSignature::FourFour)),
//     ("Resistor", (Style::Pop, TimeSignature::FourFour)),
//     ("Sheriff", (Style::Pop, TimeSignature::FourFour)),
//     ("Santeria", (Style::Reggae, TimeSignature::FourFour)),
//     ("Reffae 3", (Style::Reggae, TimeSignature::FourFour)),
//     ("Reggae 4", (Style::Reggae, TimeSignature::FourFour)),
//     ("Reggae 5", (Style::Reggae, TimeSignature::FourFour)),
//     ("Reggae 6P", (Style::Reggae, TimeSignature::FourFour)),
//     ("Reggae 7P", (Style::Reggae, TimeSignature::FourFour)),
//     ("Elec 1", (Style::Electronic, TimeSignature::FourFour)),
//     ("Elec 2", (Style::Electronic, TimeSignature::FourFour)),
//     ("Elec 3", (Style::Electronic, TimeSignature::FourFour)),
//     ("Elec-EDM", (Style::Electronic, TimeSignature::FourFour)),
//     ("Elec-tech", (Style::Electronic, TimeSignature::FourFour)),
// ]);
//  # Name          TS      Style
// 01 STANDARD	     2/4	ROCK
// 02 SWING ROCK	12/8	ROCK
// 03 POWER BEAT	 4/4	ROCK
// 04 SMOOTH	     4/4	ROCK
// 05 MEGA DRIVE	 4/4	ROCK
// 06 HARD ROCK	     4/4	ROCK
// 07 BOOGIE	     4/4	ROCK
// 01 WALK LINE	     4/4	CTRY
// 02 BLUE GRASS	 4/4	CTRY
// 03 COUNTRY	     4/4	CTRY
// 04 WALTZ	         3/4	CTRY
// 05 TRAIN	         4/4	CTRY
// 06 CTRY ROCK	     4/4	CTRY
// 07 SLOWLY	     4/4	CTRY
// 01 SLOW BLUES	12/8	BLUES
// 02 CHICAGO	     4/4	BLUES
// 03 R&B	         4/4	BLUES
// 04 BLUES ROCK	 4/4	BLUES
// 05 ROAD TRAIN	12/8	BLUES
// 06 SHUFFLE	     4/4	BLUES
// 01 2X BASS	     4/4	METAL
// 02 CLOSE BEAT	 4/4	METAL
// 03 HEAVY BASS	 4/4	METAL
// 04 FAST	         4/4	METAL
// 05 HOLY CASE	     4/4	METAL
// 06 OPEN HAT	     4/4	METAL
// 07 EPIC	         4/4	METAL
// 01 BOUNS	         4/4	FUNK
// 02 EAST COAST	 4/4	FUNK
// 03 NEW MANN	     4/4	FUNK
// 04 R&B FUNK	     4/4	FUNK
// 06 SOUL	         4/4	FUNK
// 07 UNCLE JAM	     4/4	FUNK
// 05 80 FUNK	     4/4	FUNK
// 01 BLUES JAZZ	 4/4	JAZZ
// 02 CLASSIC 1	     4/4	JAZZ
// 03 CLASSIC 2	     4/4	JAZZ
// 04 EASY JAZZ	     4/4	JAZZ
// 05 FAST	         4/4	JAZZ
// 06 WALKING	     4/4	JAZZ
// 07 SMOOTH	     4/4	JAZZ
// 01 BLUESY	     6/8	BALLAD
// 02 GROOVES	     4/4	BALLAD
// 03 BALD ROCK	     4/4	BALLAD
// 04 SLOW ROCK	     4/4	BALLAD
// 05 TUTORIAL	     4/4	BALLAD
// 06 R&B BALD	     4/4	BALLAD
// 07 GOSPEL	     4/4	BALLAD
// 01 BEACH SIDE	     2/4	POP
// 02 BIG CITY	     4/4	POP
// 03 FUNKY POP	     4/4	POP
// 04 MODERN	     4/4	POP
// 05 SCHOOL POP	 4/4	POP
// 06 MOTOWN	     4/4	POP
// 07 RESISTOR	     4/4	POP
// 01 SHERRIFF	     4/4	POP
// 02 SANTERIA	     4/4	REGGAE
// 03 REGGAE 3	     4/4	REGGAE
// 04 REGGAE 4	     4/4	REGGAE
// 05 REGGAE 5	     4/4	REGGAE
// 06 REGAE 6P	     4/4	REGGAE
// 07 REGGAE 7P	     4/4	REGGAE
// 01 ELEC 1	     4/4	ELECTRONIC
// 02 ELEC 2	     4/4	ELECTRONIC
// 03 ELEC 3	     4/4	ELECTRONIC
// 04 ELEC-EDM	     4/4	ELECTRONIC
// 05 ELEC-TECH	     4/4	ELECTRONIC
