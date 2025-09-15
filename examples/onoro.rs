use cooperate::solvers::ttable_alpha_beta::TTAlphaBeta;
use onoro::{
  abstract_game::interactive::{
    bot_player::BotPlayer, human_term_player::HumanTermPlayer, term_interface::TermInterface,
  },
  player::OnoroPlayer,
  Onoro,
};
use onoro_impl::{solver::OnoroSolver, Onoro16};

fn main() {
  let player1 = HumanTermPlayer::new("Player 1".to_owned(), OnoroPlayer::new());

  let solver = OnoroSolver::new(TTAlphaBeta::new());
  let player2 = BotPlayer::new("Player 2".to_owned(), solver, 10);

  let game = Onoro16::default_start();

  let result = TermInterface::new(game, player1, player2).map(TermInterface::play);
  if let Err(err) = result {
    println!("{err}");
  }
}
