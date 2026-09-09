/*
 * move_opportunity — drag a deal to another pipeline stage.
 *
 * The update is RLS-scoped to the caller's tenant, so they can only move
 * their own opportunities.
 */
package forge.crm;

import forge.Op;
import forge.OpContext;
import forge.OpError;
import forge.Storage;
import forge.Value;

public final class MoveOpportunity {

    public record MoveOpportunityOutput(long committed) {}

    @Op("move_opportunity")
    public static MoveOpportunityOutput moveOpportunity(OpContext ctx, Value input) {
        String id = input.get("id", "");
        if (id.equals("")) {
            throw OpError.bad_request("id required");
        }
        String stageId = input.get("stage_id", "");
        if (stageId.equals("")) {
            throw OpError.bad_request("stage_id required");
        }
        Value res = Storage.mutate(Value.obj(
            "table", "opportunities",
            "op", "update",
            "filter", Value.obj("column_equals", Value.obj("column", "id", "value", id)),
            "patch", Value.obj("stage_id", stageId)
        ));
        return new MoveOpportunityOutput(res.get("committed", 0L));
    }
}
