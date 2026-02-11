mod constant;
mod types;

use std::env;

use llm::{
    LLMAPI,
    gemini::models::GeminiModel,
    traits::TextGenerationService,
    types::{AgentTextRequest, Thinking},
};
use regex::Regex;
use repository::{
    agent::{AgentRepositoryImpl, model::NewAgentReport},
    article::{ArticleRepository, ArticleRepositoryImpl},
    config::DatabaseServerConfig,
    metadata::{
        MetaRepository, MetaRepositoryImpl,
        subs::{MetaMentionedPlaceRepository, MetaThemeRepository},
    },
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::{constant::ReportType, types::AgentExtractorResponse};

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("{}=debug", env!("CARGO_CRATE_NAME")).into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let db_config = DatabaseServerConfig::from_env();
    let pool = db_config.get_pool().await;

    let article_repo = ArticleRepositoryImpl::new(pool.clone());
    let meta_repo = MetaRepositoryImpl::new(pool.clone());
    let agent_repo = AgentRepositoryImpl::new(pool.clone());

    let llm_api = LLMAPI::from_env();
    let gemini_token = env::var("GOOGLE_API_KEY").expect("Failed to load gemini key");
    let gemini_api = llm_api.authed_gemini(&gemini_token);

    let extract_prompt = include_str!("../extract_prompt.txt");

    fn clean_content(html: &str) -> String {
        let re = Regex::new(r"<[^>]+>").unwrap();
        let text_content = re.replace(html, "").trim().to_string();

        text_content
    }

    loop {
        let articles = article_repo.find_no_metadata().await.unwrap();

        let requests = articles
            .iter()
            .map(|a| {
                let prompt = extract_prompt
                    .replace("{title}", a.title.as_ref().unwrap())
                    // 본문에 정규표현식 적용
                    .replace("{content}", &clean_content(a.content.as_ref().unwrap()));

                AgentTextRequest::new("", &prompt, Thinking::Low)
            })
            .collect();

        let agent_responses = match gemini_api
            .batch_generate_text(
                GeminiModel::Gemini3FlashPreview,
                "Extractor",
                "ext",
                requests,
            )
            .await
        {
            Ok(r) => r,
            Err(e) => {
                tracing::error!("Agent Get Response Error: {:?}", e);
                continue;
            }
        };

        for i in 0..agent_responses.len() {
            if let None = &agent_responses[i] {
                continue;
            }

            let article = &articles[i];
            let agent_response = &agent_responses[i].as_ref().unwrap();

            let extract_response: AgentExtractorResponse = serde_json::from_str(
                agent_response
                    .get_content()
                    .replace("```json", "")
                    .replace("\n```", "")
                    .as_str(),
            )
            .unwrap();

            // repo save
            let new_agent_report = NewAgentReport::new(
                article.id,
                ReportType::Extract.to_str(),
                agent_response.get_content(),
            );
            agent_repo.save(new_agent_report).await.ok();

            let themes = extract_response.get_themes().clone();
            let places = extract_response.get_metioned_places().clone();

            // metadata save
            if let Ok(meta_id) = meta_repo
                .save(
                    article.id,
                    article.title.as_ref().unwrap().as_str(),
                    extract_response.into(),
                )
                .await
            {
                meta_repo
                    .save_themes(meta_id, themes.iter().map(|t| t.into()).collect())
                    .await
                    .ok();
                meta_repo
                    .save_places(meta_id, places.iter().map(|p| p.into()).collect())
                    .await
                    .ok();
            } else {
                // 에러발생한 이유 확인
                println!(
                    "================ Article Info ================:\n{}-{:?}",
                    article.id, article.title
                );
                println!(
                    "================ Agent Response ================:\n{:?}",
                    agent_response
                );
            };
        }

        if articles.len() < 1000 {
            break;
        }
    }

    println!("Hello, world!");
}
