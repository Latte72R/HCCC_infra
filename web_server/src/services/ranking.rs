use crate::entities::Ranking;
use crate::repositories::Users;

/// Get a ranking.
pub async fn get_ranking(repo: &impl Users, include_after_contest: bool) -> Ranking {
    let ranks = repo.create_ranking(include_after_contest).await;
    Ranking::new(ranks)
}
