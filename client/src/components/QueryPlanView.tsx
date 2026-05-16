import type { QueryPlan } from '../types';

interface Props {
  plan: QueryPlan;
}

export function QueryPlanView({ plan }: Props) {
  return (
    <div className="query-plan">
      <h3>検索プラン: {plan.original_query}</h3>
      <div className="site-queries">
        {plan.searxng_queries.map((q, i) => (
          <span key={i} className="query-tag">{q}</span>
        ))}
      </div>
    </div>
  );
}