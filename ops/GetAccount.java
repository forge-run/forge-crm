/*
 * get_account — the account 360 workbench. Account + its contacts + its
 * opportunities + its activity timeline, in one payload.
 */
package forge.crm;

import forge.Forge;
import forge.Op;
import forge.OpContext;
import forge.OpError;
import forge.Storage;
import forge.Value;

import java.util.List;

public final class GetAccount {

    public record GetAccountOutput(
        Value account,
        List<Value> contacts,
        List<Value> opportunities,
        List<Value> activities
    ) {}

    /** `by_id` — the point lookup, JSON null when the row is absent. */
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

    /** `related` — a child table's rows for one parent id, newest first. */
    private static List<Value> related(String table, String column, String id, long limit) {
        return Storage.rows(Storage.query(Value.obj(
            "from", table,
            "filter", Value.obj("column", column, "op", "eq", "value", id),
            "sort", Value.list(Value.obj("column", "created_at", "direction", "desc")),
            "limit", limit
        )));
    }

    /** `timeline` — the activity feed for one related record. */
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

    @Op("get_account")
    public static GetAccountOutput getAccount(OpContext ctx, Value input) {
        String id = input.get("id", "");
        if (id.equals("")) {
            throw OpError.bad_request("id required");
        }
        Value account = byId("accounts", id);
        if (account.isNull()) {
            throw OpError.not_found();
        }
        return new GetAccountOutput(
            account,
            related("contacts", "account_id", id, 200L),
            related("opportunities", "account_id", id, 200L),
            timeline("account", id)
        );
    }
}
