---
name: reviewing-ai-papers
description: Analyze AI/ML technical content (papers, articles, blog posts) and extract actionable insights. Use when the user provides a URL or document for AI/ML content analysis, asks to "review this paper", or mentions technical content in domains like RAG, embeddings, fine-tuning, prompt engineering, or LLM deployment.
---

# Reviewing AI Papers

Analyze AI/ML technical content (papers, articles, blog posts) and extract actionable insights with a practical engineering focus.

Adapted from [oaustegard/claude-skills reviewing-ai-papers](https://github.com/oaustegard/claude-skills/blob/main/reviewing-ai-papers/SKILL.md).

## Analytical Standards

- **Maintain objectivity**: Extract factual insights without amplifying source hype
- **Challenge novelty claims**: Identify what practitioners already use as baselines. Distinguish "applies existing techniques" from "genuinely new methods"
- **Separate rigor from novelty**: A well-executed study of standard techniques ≠ a methodological breakthrough
- **Confidence transparency**: Distinguish established facts, emerging trends, and speculative claims
- **Practical focus**: Prioritize insights that map to real-world engineering challenges

## Analysis Structure

### For Substantive Content

**Article Assessment** (2-3 sentences)
- Core topic and primary claims
- Credibility: author expertise, evidence quality, methodology rigor

**Prioritized Insights**
- High Priority: Directly applicable techniques or findings
- Medium Priority: Adjacent technologies worth monitoring
- Low Priority: Interesting but not immediately actionable

**Technical Evaluation**
- Distinguish novel methods from standard practice presented as innovation
- Flag implementation challenges, risks, resource requirements
- Note contradictions with established best practices

**Actionable Recommendations**
- Research deeper: Specific areas requiring investigation
- Evaluate for implementation: Techniques worth prototyping
- Monitor trends: Emerging areas to track

**Immediate Applications**
Map insights to practical use cases. Identify quick wins or prototype opportunities.

### For Thin Content

- State limitations upfront
- Extract marginal insights if any
- Recommend better alternatives if the topic matters
- Keep brief

## Output Standards

- **Conciseness**: Actionable insights, not content restatement
- **Precision**: Distinguish demonstrates/suggests/claims/speculates
- **Relevance**: Connect to practical applications or state no connection
- **Adaptive depth**: Match analysis length to content value

## Constraints

- No hype amplification
- No timeline predictions unless specifically requested
- No speculation beyond article content
- Note contradictions with other known work explicitly
- State limitations clearly on thin content
