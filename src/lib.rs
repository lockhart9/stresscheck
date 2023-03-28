use once_cell::sync::Lazy;
use serde::Deserialize;

pub static QUESTIONS: Lazy<SimpleStress> = Lazy::new(|| {
    let f = std::fs::File::open("resources/57.json").unwrap();
    let reader = std::io::BufReader::new(f);
    serde_json::from_reader(reader).unwrap()
});

#[derive(Debug, Clone, Deserialize)]
pub struct Score {
    pub score: u8,
    pub text: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Question {
    pub id: u32,
    pub text: String,
    pub reverse: bool,
    pub scores: Vec<Score>,
}

impl From<Question> for u32 {
    fn from(q: Question) -> Self {
        q.id
    }
}

#[derive(Debug, Deserialize)]
pub struct OuterQuestion {
    /// サブ教示文
    /// あなたの周りの方々についてうかがいます。最もあてはまるものに○を付けてください。
    /// の調査ブロックは3つの設問サブセットに分解され、それぞれのサブセットに教示が内包されている。
    /// 詳細 https://www.mhlw.go.jp/bunya/roudoukijun/anzeneisei12/dl/stress-check_j.pdf
    pub title: Option<String>,
    pub questions: Vec<Question>,
}

#[derive(Debug, Deserialize)]
pub struct Theme {
    /// 教示文
    pub theme: String,
    pub questions: Vec<OuterQuestion>,
}

/// ストレスチェック57設問のマスタ表現
#[derive(Debug, Deserialize)]
pub struct SimpleStress {
    pub simple_stress: Vec<Theme>,
}

impl SimpleStress {
    pub fn get(&self, index: usize) -> Option<Question> {
        self.simple_stress
            .iter()
            .flat_map(|theme| {
                theme
                    .questions
                    .iter()
                    .flat_map(|outer_question| outer_question.questions.clone())
            })
            .nth(index)
    }

    /// 設問番号を指定して設問を取得する
    pub fn question(&self, id: u32) -> Option<Question> {
        self.simple_stress
            .iter()
            .flat_map(|theme| {
                theme
                    .questions
                    .iter()
                    .flat_map(|outer_question| outer_question.questions.clone())
            })
            .find(|question| question.id == id)
    }

    /// 57設問を全て取得する
    pub fn questions(&self) -> Vec<Question> {
        self.simple_stress
            .iter()
            .flat_map(|theme| {
                theme
                    .questions
                    .iter()
                    .flat_map(|outer_question| outer_question.questions.clone())
            })
            .collect::<Vec<Question>>()
    }
}

#[derive(Debug, Clone)]
pub struct AnswerStore {
    values: [u8; 57],
    offset: usize,
}

impl Default for AnswerStore {
    fn default() -> Self {
        Self {
            values: [0; 57],
            offset: 0,
        }
    }
}

impl AnswerStore {
    /// 回答を格納する
    /// 1〜4の回答番号以外は認めない。
    pub fn push(&mut self, score: u8) -> Result<(), Error> {
        if (1..=4).contains(&score) {
            if self.offset < 57 {
                self.values[self.offset] = score;
                self.offset += 1;
                Ok(())
            } else {
                Err(Error::IllegalQuestion)
            }
        } else {
            Err(Error::IllegalAnswer)
        }
    }

    /// 設問番号を指定して回答を格納する
    pub fn insert(&mut self, question_no: u8, score: u8) -> Result<(), Error> {
        if question_no < 1 {
            return Err(Error::IllegalQuestion);
        }
        if (1..=4).contains(&score) {
            let offset: usize = (question_no - 1).into();
            if offset < 57 {
                self.values[offset] = score;
                Ok(())
            } else {
                Err(Error::IllegalQuestion)
            }
        } else {
            Err(Error::IllegalAnswer)
        }
    }

    /// 合計点数方式
    ///
    /// ○ まず、労働者が記入又は入力した調査票を元に、合計点数を算出します。
    ///
    /// 合計点数を算出する時に、もっとも気をつけなければいけない点は、質
    /// 問の一部に、質問の聞き方により、点数が低いほどストレスが高いと評価
    /// すべき質問が混ざっていることです。こうした質問の場合は、回答のあっ
    /// た点数を逆転させて足し合わせていく必要があります。
    ///
    /// 具体的には、職業性ストレス簡易調査票（57 項目）の質問のうち、領域
    /// 「Ａ」の１～７、11～13、15、領域「Ｂ」の１～３（次ページの回答例
    /// の の枠内）の質問項目については、点数が低いほどストレスが高い
    /// という評価になるため、回答のあった点数に応じて、１⇒４、２⇒３、３
    /// ⇒２、４⇒１に置き換えなおし、点数を足していく必要があります。
    /// ○ このようにしてＡ、Ｂ、Ｃの領域ごとに合計点数を算出したら、次に高
    /// ストレス者を選定する数値基準に照らし合わせます。
    ///
    /// マニュアルにおいて、高ストレス者を選定する評価基準の設定例（その
    /// １）では、職業性ストレス簡易調査票（57 項目）を使用する場合、以下の
    /// いずれかを満たす場合に、高ストレス者と選定することとなっています。
    ///
    /// ㋐ 領域Ｂの合計点数が 77 点以上（最高点は４×29＝116 点）であること
    /// ㋑ 領域ＡとＣの合算の合計点数が76点以上（最高点は４×17＋４×９＝104
    /// 点）であり、かつ領域Ｂの合計点数が 63 点以上であること
    pub fn to_sumup_score(&self) -> Result<SumupScore, Error> {
        if self.values.iter().any(|&value| value == 0) {
            return Err(Error::NotFullfilled);
        }
        let values = self
            .values
            .iter()
            .enumerate()
            .map(|(index, &value)| reverse_if((index + 1, value)))
            .collect::<Vec<u8>>();
        Ok(SumupScore {
            sum_a: values.iter().take(17).sum(),
            sum_b: values.iter().skip(17).take(29).sum(),
            sum_c: values.iter().skip(46).take(9).sum(),
        })
    }

    /// ○ 素点換算表では、職業性ストレス簡易調査票の質問項目が、いくつかの
    /// まとまりごとに尺度としてまとめられ、計算方法が示されています。例え
    /// ば、質問項目の１～３は、次ページの「素点換算表に基づく評価点の算出
    /// 方法」の表の一番上にある「心理的な仕事の負担（量）」という尺度にまと
    /// められます。
    ///
    /// ○ 尺度ごとの計算結果を素点換算表に当てはめ、５段階評価の評価点を出
    /// します。
    /// 【素点換算表に当てはめて評価点を出す場合の留意点】
    /// ・ 素点換算表では評価点が低いほどストレスの程度が高いという評価になります。
    /// ・ １の場合と同様に、尺度によって、ストレスの程度の意味合いが逆になるもの（例え
    /// ば、「心理的な仕事の負担（量）」が「高い／多い」のと、「仕事のコントロール度」が
    /// 「高い／多い」のとでは意味合いが逆になる）がありますが、その場合は素点換算表の
    /// 評価点が予め逆向きに設定されています。具体的には、次ページの「素点換算表に基づ
    /// く評価点の算出方法」の表でみると、「心理的な仕事の負担（量）」の尺度と、「仕事の
    /// コントロール度」の尺度では、評価点の並び方が逆向きになっていることが分かります
    /// （灰色に色づけされた欄でみていけば、灰色の欄が最もストレスの程度が高いという意
    /// 味になります）。
    ///
    /// ○ このようにして求めた評価点を領域「Ａ」、「Ｂ」、「Ｃ」ごとに合計し、
    /// 高ストレス者を選定する数値基準に照らし合わせます。
    ///
    /// マニュアルにおいて、素点換算表を用いる際の高ストレス者を選定する
    /// 評価基準の設定例（その２）では、以下のいずれかを満たす場合に、高ス
    /// トレス者と選定することとなっています。
    ///
    /// ㋐ 領域Ｂの評価点の合計が 12 点以下（最低点は１×６＝６点）であること
    /// ㋑ 領域ＡとＣの合算の評価点の合計が 26 点以下（最低点は１×９＋１×３
    /// ＝12 点）であり、かつ領域Ｂの評価点の合計が 17 点以下であること
    pub fn to_conversion_score(&self) -> Result<ConversionScore, Error> {
        if self.values.iter().any(|&value| value == 0) {
            return Err(Error::NotFullfilled);
        }
        IntermediateConversionScore {
            mental_work_stress_volume: 15 - self.values.iter().take(3).sum::<u8>(),
            mental_work_stress_quality: 15 - self.values.iter().skip(3).take(3).sum::<u8>(),
            aware_physical_stress: 5 - self.values.get(6).ok_or(Error::IllegalAnswer)?,
            work_people_stress: 10 - self.values.iter().skip(11).take(2).sum::<u8>()
                + self.values.get(13).ok_or(Error::IllegalAnswer)?,
            work_env_stress: 5 - self.values.get(14).ok_or(Error::IllegalAnswer)?,
            work_control: 15 - self.values.iter().skip(7).take(3).sum::<u8>(),
            skill_apply: (*self.values.get(10).ok_or(Error::IllegalAnswer)?),
            work_apply: 5 - self.values.get(15).ok_or(Error::IllegalAnswer)?,
            decent_work: 5 - self.values.get(16).ok_or(Error::IllegalAnswer)?,
            vitality: self.values.iter().skip(17).take(3).sum::<u8>(),
            iraira: self.values.iter().skip(20).take(3).sum::<u8>(),
            tired: self.values.iter().skip(23).take(3).sum::<u8>(),
            anxious: self.values.iter().skip(26).take(3).sum::<u8>(),
            depressed: self.values.iter().skip(29).take(6).sum::<u8>(),
            physical_complaint: self.values.iter().skip(35).take(11).sum::<u8>(),
            boss_support: 15
                - (self.values.get(46).ok_or(Error::IllegalAnswer)?
                    + self.values.get(49).ok_or(Error::IllegalAnswer)?
                    + self.values.get(52).ok_or(Error::IllegalAnswer)?),
            colleague_support: 15
                - (self.values.get(47).ok_or(Error::IllegalAnswer)?
                    + self.values.get(50).ok_or(Error::IllegalAnswer)?
                    + self.values.get(53).ok_or(Error::IllegalAnswer)?),
            family_support: 15
                - (self.values.get(48).ok_or(Error::IllegalAnswer)?
                    + self.values.get(51).ok_or(Error::IllegalAnswer)?
                    + self.values.get(54).ok_or(Error::IllegalAnswer)?),
        }
        .try_into()
    }
}

fn reverse_if(score: (usize, u8)) -> u8 {
    match score.0 {
        ref id if (1..=7).contains(id) => 5 - score.1,
        ref id if (11..=13).contains(id) => 5 - score.1,
        15 => 5 - score.1,
        ref id if (18..=20).contains(id) => 5 - score.1,
        _ => score.1,
    }
}

pub trait Stress {
    fn scores(&self) -> (u8, u8, u8);
    fn has_stress(&self) -> bool;
}

#[derive(Debug)]
pub struct SumupScore {
    sum_a: u8,
    sum_b: u8,
    sum_c: u8,
}

impl Stress for SumupScore {
    fn has_stress(&self) -> bool {
        self.sum_b >= 77 || (self.sum_a + self.sum_c >= 76 && self.sum_b >= 63)
    }

    fn scores(&self) -> (u8, u8, u8) {
        (self.sum_a, self.sum_b, self.sum_c)
    }
}

pub struct IntermediateConversionScore {
    /// 心理的な仕事の負担（量）
    mental_work_stress_volume: u8,
    /// 心理的な仕事の負担（質）
    mental_work_stress_quality: u8,
    /// 自覚的な身体的負担度
    aware_physical_stress: u8,
    /// 職場の対人関係でのストレス
    work_people_stress: u8,
    /// 職場環境によるストレス
    work_env_stress: u8,
    ///
    /// 仕事のコントロール
    work_control: u8,
    /// 技能の活用度
    skill_apply: u8,
    /// 仕事の適正度
    work_apply: u8,
    /// 働きがい
    decent_work: u8,

    /// 活気
    vitality: u8,

    /// イライラ感
    iraira: u8,
    /// 疲労感
    tired: u8,
    /// 不安感
    anxious: u8,
    /// 抑うつ感
    depressed: u8,
    /// 身体愁訴
    physical_complaint: u8,
    ///
    /// 上司からのサポート
    boss_support: u8,
    /// 同僚からのサポート
    colleague_support: u8,
    /// 家族友人からのサポート
    family_support: u8,
}

impl TryFrom<IntermediateConversionScore> for ConversionScore {
    type Error = Error;

    fn try_from(score: IntermediateConversionScore) -> Result<Self, Self::Error> {
        Ok(ConversionScore {
            mental_work_stress_volume: match score.mental_work_stress_volume {
                ref score if (3..=5).contains(score) => 5,
                ref score if (6..=7).contains(score) => 4,
                ref score if (8..=9).contains(score) => 3,
                ref score if (10..=11).contains(score) => 2,
                12 => 1,
                _ => return Err(Error::IllegalAnswer),
            },
            mental_work_stress_quality: match score.mental_work_stress_quality {
                ref score if (3..=5).contains(score) => 5,
                ref score if (6..=7).contains(score) => 4,
                ref score if (8..=9).contains(score) => 3,
                ref score if (10..=11).contains(score) => 2,
                12 => 1,
                _ => return Err(Error::IllegalAnswer),
            },
            aware_physical_stress: match score.aware_physical_stress {
                1 => 4,
                2 => 3,
                3 => 2,
                4 => 1,
                _ => return Err(Error::IllegalAnswer),
            },
            work_people_stress: match score.work_people_stress {
                3 => 5,
                ref score if (4..=5).contains(score) => 4,
                ref score if (6..=7).contains(score) => 3,
                ref score if (8..=9).contains(score) => 2,
                ref score if (10..=12).contains(score) => 1,
                _ => return Err(Error::IllegalAnswer),
            },
            work_env_stress: match score.work_env_stress {
                1 => 4,
                2 => 3,
                3 => 2,
                4 => 1,
                _ => return Err(Error::IllegalAnswer),
            },
            work_control: match score.work_control {
                ref score if (3..=4).contains(score) => 1,
                ref score if (5..=6).contains(score) => 2,
                ref score if (7..=8).contains(score) => 3,
                ref score if (9..=10).contains(score) => 4,
                ref score if (11..=12).contains(score) => 5,
                _ => return Err(Error::IllegalAnswer),
            },
            skill_apply: match score.skill_apply {
                1 => 1,
                2 => 2,
                3 => 3,
                4 => 4,
                _ => return Err(Error::IllegalAnswer),
            },
            work_apply: match score.work_apply {
                1 => 1,
                2 => 2,
                3 => 3,
                4 => 5,
                _ => return Err(Error::IllegalAnswer),
            },
            decent_work: match score.decent_work {
                1 => 1,
                2 => 2,
                3 => 3,
                4 => 5,
                _ => return Err(Error::IllegalAnswer),
            },
            vitality: match score.vitality {
                3 => 1,
                ref score if (4..=5).contains(score) => 2,
                ref score if (6..=7).contains(score) => 3,
                ref score if (8..=9).contains(score) => 4,
                ref score if (10..=12).contains(score) => 5,
                _ => return Err(Error::IllegalAnswer),
            },
            iraira: match score.iraira {
                3 => 5,
                ref score if (4..=5).contains(score) => 4,
                ref score if (6..=7).contains(score) => 3,
                ref score if (8..=9).contains(score) => 2,
                ref score if (10..=12).contains(score) => 1,
                _ => return Err(Error::IllegalAnswer),
            },
            tired: match score.tired {
                3 => 5,
                4 => 4,
                ref score if (5..=7).contains(score) => 3,
                ref score if (8..=10).contains(score) => 2,
                ref score if (11..=12).contains(score) => 1,
                _ => return Err(Error::IllegalAnswer),
            },
            anxious: match score.anxious {
                3 => 5,
                4 => 4,
                ref score if (5..=7).contains(score) => 3,
                ref score if (8..=9).contains(score) => 2,
                ref score if (10..=12).contains(score) => 1,
                _ => return Err(Error::IllegalAnswer),
            },
            depressed: match score.depressed {
                6 => 5,
                ref score if (7..=8).contains(score) => 4,
                ref score if (9..=12).contains(score) => 3,
                ref score if (13..=16).contains(score) => 2,
                ref score if (17..=24).contains(score) => 1,
                _ => return Err(Error::IllegalAnswer),
            },
            physical_complaint: match score.physical_complaint {
                11 => 5,
                ref score if (12..=15).contains(score) => 4,
                ref score if (16..=21).contains(score) => 3,
                ref score if (22..=26).contains(score) => 2,
                ref score if (27..=44).contains(score) => 1,
                _ => return Err(Error::IllegalAnswer),
            },
            boss_support: match score.boss_support {
                ref score if (3..=4).contains(score) => 1,
                ref score if (5..=6).contains(score) => 2,
                ref score if (7..=8).contains(score) => 3,
                ref score if (9..=10).contains(score) => 4,
                ref score if (11..=12).contains(score) => 5,
                _ => return Err(Error::IllegalAnswer),
            },
            colleague_support: match score.colleague_support {
                ref score if (3..=5).contains(score) => 1,
                ref score if (6..=7).contains(score) => 2,
                ref score if (8..=9).contains(score) => 3,
                ref score if (10..=11).contains(score) => 4,
                12 => 5,
                _ => return Err(Error::IllegalAnswer),
            },
            family_support: match score.family_support {
                ref score if (3..=6).contains(score) => 1,
                ref score if (7..=8).contains(score) => 2,
                9 => 3,
                ref score if (10..=11).contains(score) => 4,
                12 => 5,
                _ => return Err(Error::IllegalAnswer),
            },
        })
    }
}

pub struct ConversionScore {
    /// 心理的な仕事の負担（量）
    mental_work_stress_volume: u8,
    /// 心理的な仕事の負担（質）
    mental_work_stress_quality: u8,
    /// 自覚的な身体的負担度
    aware_physical_stress: u8,
    /// 職場の対人関係でのストレス
    work_people_stress: u8,
    /// 職場環境によるストレス
    work_env_stress: u8,
    ///
    /// 仕事のコントロール
    work_control: u8,
    /// 技能の活用度
    skill_apply: u8,
    /// 仕事の適正度
    work_apply: u8,
    /// 働きがい
    decent_work: u8,

    /// 活気
    vitality: u8,

    /// イライラ感
    iraira: u8,
    /// 疲労感
    tired: u8,
    /// 不安感
    anxious: u8,
    /// 抑うつ感
    depressed: u8,
    /// 身体愁訴
    physical_complaint: u8,
    ///
    /// 上司からのサポート
    boss_support: u8,
    /// 同僚からのサポート
    colleague_support: u8,
    /// 家族友人からのサポート
    family_support: u8,
}

impl Stress for ConversionScore {
    fn has_stress(&self) -> bool {
        let (sum_a, sum_b, sum_c) = self.scores();
        sum_b <= 12 || (sum_a + sum_c <= 26 && sum_b <= 17)
    }

    fn scores(&self) -> (u8, u8, u8) {
        (
            self.mental_work_stress_volume
                + self.mental_work_stress_quality
                + self.aware_physical_stress
                + self.work_people_stress
                + self.work_env_stress
                + self.work_control
                + self.skill_apply
                + self.work_apply
                + self.decent_work,
            self.vitality
                + self.iraira
                + self.tired
                + self.anxious
                + self.depressed
                + self.physical_complaint,
            self.boss_support + self.colleague_support + self.family_support,
        )
    }
}

#[derive(Debug)]
pub enum Error {
    /// 57設問ではない
    IllegalQuestion,
    /// 回答選択肢が違反
    IllegalAnswer,
    /// 回答欠落
    NotFullfilled,
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_get() {
        assert_eq!(Some(1), QUESTIONS.get(0).map(|q| q.id));
        assert_eq!(Some(57), QUESTIONS.get(56).map(|q| q.id));
        assert_eq!(None, QUESTIONS.get(57).map(|q| q.id));
    }

    #[test]
    fn test_question() {
        assert_eq!(Some(1), QUESTIONS.question(1).map(|q| q.id));
        assert_eq!(Some(57), QUESTIONS.question(57).map(|q| q.id));
        assert_eq!(None, QUESTIONS.question(58).map(|q| q.id));
    }

    #[test]
    fn test_questions() {
        let questions = QUESTIONS.questions();
        assert_eq!(questions.len(), 57);
        assert_eq!(questions.get(0).map(|q| q.id), Some(1));
        assert_eq!(questions.get(56).map(|q| q.id), Some(57));
        assert_eq!(questions.get(57).map(|q| q.id), None);
    }

    #[test]
    fn test_reverse_if() {
        assert_eq!(reverse_if((1, 1)), 4);
        assert_eq!(reverse_if((2, 2)), 3);
        assert_eq!(reverse_if((3, 3)), 2);
        assert_eq!(reverse_if((4, 4)), 1);
        assert_eq!(reverse_if((7, 1)), 4);
        assert_eq!(reverse_if((8, 3)), 3);
        assert_eq!(reverse_if((10, 4)), 4);
        assert_eq!(reverse_if((11, 1)), 4);
        assert_eq!(reverse_if((12, 2)), 3);
        assert_eq!(reverse_if((13, 3)), 2);
        assert_eq!(reverse_if((14, 4)), 4);
        assert_eq!(reverse_if((15, 1)), 4);
        assert_eq!(reverse_if((17, 1)), 1);
        assert_eq!(reverse_if((18, 2)), 3);
        assert_eq!(reverse_if((19, 3)), 2);
        assert_eq!(reverse_if((20, 4)), 1);
        assert_eq!(reverse_if((21, 1)), 1);
        assert_eq!(reverse_if((57, 2)), 2);
    }

    #[test]
    fn test_answer_store_low() {
        let mut store = AnswerStore::default();
        for _ in 0..57 {
            assert!(store.push(1).is_ok());
        }
        let score = store.to_sumup_score().unwrap();
        assert_eq!(score.sum_a, 50);
        assert_eq!(score.sum_b, 38);
        assert_eq!(score.sum_c, 9);
    }

    #[test]
    fn test_answer_store_high() {
        let mut store = AnswerStore::default();
        for _ in 0..57 {
            assert!(store.push(4).is_ok());
        }
        let score = store.to_sumup_score().unwrap();
        assert_eq!(score.sum_a, 35);
        assert_eq!(score.sum_b, 107);
        assert_eq!(score.sum_c, 4 * 9);
    }

    #[test]
    fn test_answer_not_fullfilled() {
        let mut store = AnswerStore::default();
        for _ in 0..56 {
            assert!(store.push(1).is_ok());
        }
        assert!(store.to_sumup_score().is_err());
    }

    #[test]
    fn test_answer_exceeded() {
        let mut store = AnswerStore::default();
        for _ in 0..57 {
            assert!(store.push(1).is_ok());
        }
        assert!(store.push(1).is_err());
    }

    #[test]
    fn test_insert() {
        let mut store = AnswerStore::default();
        assert!(store.insert(0, 1).is_err());
        assert!(store.insert(1, 1).is_ok());
        assert!(store.insert(57, 1).is_ok());
        assert!(store.insert(58, 1).is_err());
        assert!(store.insert(10, 5).is_err());
    }

    #[test]
    fn test_sumup_score_stress() {
        let score = SumupScore {
            sum_a: 17,
            sum_b: 76,
            sum_c: 9,
        };
        assert!(!score.has_stress());

        let score = SumupScore {
            sum_a: 17,
            sum_b: 77,
            sum_c: 9,
        };
        assert!(score.has_stress());

        let score = SumupScore {
            sum_a: 46,
            sum_b: 62,
            sum_c: 30,
        };
        assert!(!score.has_stress());

        let score = SumupScore {
            sum_a: 46,
            sum_b: 63,
            sum_c: 30,
        };
        assert!(score.has_stress());

        let score = SumupScore {
            sum_a: 45,
            sum_b: 63,
            sum_c: 30,
        };
        assert!(!score.has_stress());
    }

    #[test]
    fn test_conversion_score() {
        let mut store = AnswerStore::default();
        for _ in 0..57 {
            assert!(store.push(1).is_ok());
        }
        let store = store.to_conversion_score().unwrap();

        // 22
        assert_eq!(store.mental_work_stress_volume, 1);
        assert_eq!(store.mental_work_stress_quality, 1);
        assert_eq!(store.aware_physical_stress, 1);
        assert_eq!(store.work_people_stress, 2);
        assert_eq!(store.work_env_stress, 1);
        assert_eq!(store.work_control, 5);
        assert_eq!(store.skill_apply, 1);
        assert_eq!(store.work_apply, 5);
        assert_eq!(store.decent_work, 5);

        // 26
        assert_eq!(store.vitality, 1);
        assert_eq!(store.iraira, 5);
        assert_eq!(store.tired, 5);
        assert_eq!(store.anxious, 5);
        assert_eq!(store.depressed, 5);
        assert_eq!(store.physical_complaint, 5);

        // 15
        assert_eq!(store.boss_support, 5);
        assert_eq!(store.colleague_support, 5);
        assert_eq!(store.colleague_support, 5);

        assert_eq!(store.scores(), (22, 26, 15));

        assert!(!store.has_stress());
    }

    #[test]
    fn test_conversion_score_answer_not_fullfilled() {
        let mut store = AnswerStore::default();
        assert!(store.push(1).is_ok());
        assert!(store.to_conversion_score().is_err());
    }
}
