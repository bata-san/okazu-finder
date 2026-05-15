import type { QueryPlan } from '../types';

interface Props {
  plan: QueryPlan;
}

export function QueryPlanView({ plan }: Props) {
  const sites = Object.entries(plan.site_queries).filter(
    ([, queries]) => queries.length > 0,
  );

  return (
    <div className="query-plan">
      <h3>検索プラン: {plan.original_query}</h3>
      <div className="site-tags">
        {sites.map(([site, queries]) => (
          <div key={site} className="site-group">
            <div className="site-name">{site}</div>
            <div className="site-queries">
              {queries.slice(0, 3).join(' / ')}
              {queries.length > 3 && ` +${queries.length - 3}`}
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}