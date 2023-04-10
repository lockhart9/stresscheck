use std::io::stdin;

use simple_stresscheck::Stress;
use simple_stresscheck::{Error, StressCheck, QUESTIONS};

fn main() {
    let mut buffer = String::new();
    let mut store = StressCheck::default();

    for theme in &QUESTIONS.simple_stress {
        println!("{}", theme.theme);
        for outer_question in &theme.questions {
            if let Some(ref title) = outer_question.title {
                println!("{}", title);
            }
            for inner_question in &outer_question.questions {
                println!("{}", inner_question.text);
                for score in &inner_question.scores {
                    print!("  {} => {}", score.score, score.text);
                }
                loop {
                    println!();
                    stdin().read_line(&mut buffer).unwrap();
                    if store_answer(buffer.trim(), &mut store).is_err() {
                        println!("回答は半角英数1〜4で入力してください。");
                        buffer.clear();
                    } else {
                        buffer.clear();
                        break;
                    }
                }
                println!();
            }
        }
    }

    let score = store.to_sumup_score().unwrap();
    match score.has_stress() {
        true => println!("あなたは高ストレス状態です。"),
        false => println!("あなたは高ストレスではありません。"),
    }

    // dbg!("{} {}", score, store);
}

fn store_answer(value: &str, store: &mut StressCheck) -> Result<(), Error> {
    let value = value.parse::<u8>().map_err(|_| Error::IllegalAnswer)?;
    store.push(value)?;
    Ok(())
}
