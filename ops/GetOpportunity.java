/*
 * get_opportunity — the deal workbench. Opp + its account + the full stage
 * list (for the move/close control) + its activity timeline.
 */
package forge.crm;

import forge.Forge;
import forge.Op;
import forge.OpContext;
import forge.OpError;
import forge.Storage;
import forge.Value;

import java.util.List;

public final class GetOpportunity {

    public record GetOpportunityOutput(
        Value opportunity,
        Value account,
        List<Value> stages,
        List<Value> activities
    ) {}

    private static Value byId(String table, String id) {
        List<Value> found = Storage.rows(Storage.query(Value.obj(
            "from", table,
            "filter", Value.obj("column", "id", "op", "eq", "value", id),
            "limit", 1L
        )));
        if (found.isEmpty()) {
            return Value.none();
        }
        return Forge.nth(found, 0L);
    }

    private static List<Value> timeline(String relatedType, String relatedId) {
        return Storage.rows(Storage.query(Value.obj(
            "from", "activities",
            "filter", Value.obj("and", Value.list(
                Value.obj("column", "related_type", "op", "eq", "value", relatedType),
                Value.obj("column", "related_id", "op", "eq", "value", relatedId)
            )),
            "sort", Value.list(Value.obj("column", "created_at", "direction", "desc")),
            "limit", 50L
        )));
    }

    @Op("get_opportunity")
    public static GetOpportunityOutput getOpportunity(OpContext ctx, Value input) {
        String id = input.get("id", "");
        if (id.equals("")) {
            throw OpError.bad_request("id required");
        }
        Value opportunity = byId("opportunities", id);
        if (opportunity.isNull()) {
            throw OpError.not_found();
        }
        Value account = Value.none();
        String accountId = opportunity.get("account_id", "");
        if (!accountId.equals("")) {
            account = byId("accounts", accountId);
        }
        List<Value> stages = Storage.rows(Storage.query(Value.obj(
            "from", "pipeline_stages",
            "sort", Value.list(Value.obj("column", "position", "direction", "asc")),
            "limit", 50L
        )));
        return new GetOpportunityOutput(opportunity, account, stages, timeline("opportunity", id));
    }
}
