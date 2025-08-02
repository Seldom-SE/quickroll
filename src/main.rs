use chumsky::{extra::Err, prelude::*};
use rand::random_range;
use serenity::{all::Message, async_trait, prelude::*};
use text::int;

#[derive(Debug)]
enum Expr {
    Dice { count: u32, size: u32 },
    Literal(i32),
}

#[derive(Debug)]
struct Roll {
    exprs: Vec<Expr>,
    advantageousness: i32,
}

fn parser<'a>() -> impl Parser<'a, &'a str, Roll, Err<Rich<'a, char>>> {
    let num = int(10).map(|int: &str| int.parse().unwrap());
    let signed = just('-').or_not().then(num).map(|(neg, num)| {
        if neg.is_some() {
            -(num as i32)
        } else {
            num as i32
        }
    });

    let ending = just('a')
        .repeated()
        .at_least(1)
        .collect::<Vec<_>>()
        .map(|r#as| r#as.len() as i32)
        .or(just('d')
            .repeated()
            .at_least(1)
            .collect::<Vec<_>>()
            .map(|ds| -(ds.len() as i32)))
        .or_not()
        .map(|advantageousness| advantageousness.unwrap_or_default())
        .then_ignore(just('/').then(any().repeated()).or_not())
        .then_ignore(end());

    just('r')
        .ignore_then(
            signed
                .map(|num| vec![Expr::Dice { count: 1, size: 20 }, Expr::Literal(num)])
                .then(ending)
                .or(num
                    .or_not()
                    .then(just('d').ignore_then(num))
                    .map(|(count, size)| {
                        if let Some(count) = count {
                            Expr::Dice { count, size }
                        } else {
                            Expr::Dice { count: 1, size }
                        }
                    })
                    .or(signed.map(Expr::Literal))
                    .separated_by(just('+'))
                    .at_least(1)
                    .collect::<Vec<Expr>>()
                    .then(ending))
                .or(ending.map(|advantageousness| {
                    (vec![Expr::Dice { count: 1, size: 20 }], advantageousness)
                })),
        )
        .map(|(exprs, advantageousness)| Roll {
            exprs,
            advantageousness,
        })
}

struct Maximality {
    max: bool,
    min: bool,
}

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        match parser().parse(&msg.content).into_result() {
            Ok(roll) => {
                if roll.exprs.iter().any(|expr| {
                    if let &Expr::Dice { count, size } = expr {
                        count == 0 && size == 0
                    } else {
                        false
                    }
                }) {
                    return;
                }

                let rolls = (0..=roll.advantageousness.abs()).map(|_| {
                roll.exprs
                    .iter()
                    .map(|expr| match *expr {
                        Expr::Dice { count, size } => {
                            (0..count).map(|_| random_range(1..=size)).fold(
                                (
                                    0,
                                    String::new(),
                                    Maximality {
                                        max: true,
                                        min: true,
                                    },
                                    0,
                                ),
                                |(sum, string, mut maximality, count), roll| {
                                    let mut crit = false;

                                    if roll > 1 {
                                        maximality.min = false;
                                        crit ^= true;
                                    }

                                    if roll < size {
                                        maximality.max = false;
                                        crit ^= true;
                                    }

                                    (
                                        sum + roll as i32,
                                        format!(
                                            "{string}{}{}",
                                            if string.is_empty() { "" } else { " " },
                                            if crit {
                                                format!("**{roll}**")
                                            } else {
                                                roll.to_string()
                                            }
                                        ),
                                        maximality,
                                        count + 1,
                                    )
                                },
                            )
                        }
                        Expr::Literal(literal) => (
                            literal,
                            literal.to_string(),
                            Maximality {
                                max: true,
                                min: true,
                            },
                            1,
                        ),
                    })
                    .fold(
                        (
                            0,
                            String::new(),
                            Maximality {
                                max: true,
                                min: true,
                            },
                            0,
                        ),
                        |(sum, string, maximality, count), (expr_sum, expr, expr_maximality, expr_count)| {
                            (
                                sum + expr_sum,
                                format!(
                                    "{string}{}{expr}",
                                    if string.is_empty() { "" } else { " + " },
                                ),
                                Maximality {
                                    max: maximality.max && expr_maximality.max,
                                    min: maximality.min && expr_maximality.min,
                                },
                                count + expr_count,
                            )
                        },
                    )
            }).collect::<Vec<_>>();

                let (taken, _) = rolls
                    .iter()
                    .enumerate()
                    .max_by_key(|(_, (sum, _, _, _))| {
                        if roll.advantageousness >= 0 {
                            *sum
                        } else {
                            -*sum
                        }
                    })
                    .unwrap();

                if let Err(err) = msg
                    .reply(
                        &ctx.http,
                        rolls.into_iter().enumerate().fold(
                            #[cfg(feature = "debug")]
                            format!("`{roll:?}`"),
                            #[cfg(not(feature = "debug"))]
                            String::new(),
                            |string, (index, (sum, roll, maximality, count))| {
                                let delim = if index == taken { "" } else { "~~" };

                                format!(
                                    "{string}{}{delim}{roll}{}{delim}",
                                    if string.is_empty() { "" } else { " " },
                                    if count > 1 {
                                        format!(
                                            " = {}",
                                            if maximality.max != maximality.min {
                                                format!("**{sum}**")
                                            } else {
                                                sum.to_string()
                                            }
                                        )
                                    } else {
                                        String::new()
                                    }
                                )
                            },
                        ),
                    )
                    .await
                {
                    eprintln!("Error sending message: {err:?}");
                }
            }
            Err(_errs) =>
            {
                #[cfg(feature = "debug")]
                for err in _errs {
                    eprintln!("{err}");
                }
            }
        }
    }
}

#[tokio::main]
async fn main() {
    let token = include_str!("../TOKEN.txt");
    let intents = GatewayIntents::GUILD_MESSAGES | GatewayIntents::MESSAGE_CONTENT;

    // Create a new instance of the Client, logging in as a bot.
    let mut client = Client::builder(token, intents)
        .event_handler(Handler)
        .await
        .expect("Err creating client");

    // Start listening for events by starting a single shard
    if let Err(why) = client.start().await {
        eprintln!("Client error: {why:?}");
    }
}
