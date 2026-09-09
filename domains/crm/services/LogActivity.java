/*
 * log_activity — record a call / email / meeting / note against a record.
 */
package forge.crm;

import forge.Forge;
import forge.Op;
import forge.OpContext;
import forge.OpError;
import forge.Storage;
import forge.Value;

import java.util.List;

public final class LogActivity {

    public record LogActivityOutput(Value id, long committed) {}

    private static LogActivityOutput insert(String table, Value row, String uid, Value input) {
        String tenant = input.get("tenant_id", "");
        if (tenant.equals("")) {
            throw OpError.forbidden();
        }
        if (uid.equals("")) {
            throw OpError.forbidden();
        }
        row.set("tenant_id", tenant);
        row.set("owner_id", uid);
        Value res = Storage.mutate(Value.obj("table", table, "op", "insert", "rows", Value.list(row)));
        List<Value> ids = res.get("ids", List.of());
        Value id = Value.of("");
        if (!ids.isEmpty()) {
            id = Forge.nth(ids, 0L);
        }
        return new LogActivityOutput(id, res.get("committed", 0L));
    }

    @Op("log_activity")
    public static LogActivityOutput logActivity(OpContext ctx, Value input) {
        String subject = input.get("subject", "");
        if (subject.equals("")) {
            throw OpError.bad_request("subject required");
        }
        String kind = input.get("kind", "");
        if (kind.equals("")) {
            kind = "note";
        }
        Value row = Value.obj("subject", subject, "kind", kind);
        String body = input.get("body", "");
        if (!body.equals("")) {
            row.set("body", body);
        }
        String relatedType = input.get("related_type", "");
        if (!relatedType.equals("")) {
            row.set("related_type", relatedType);
        }
        String relatedId = input.get("related_id", "");
        if (!relatedId.equals("")) {
            row.set("related_id", relatedId);
        }
        String dueAt = input.get("due_at", "");
        if (!dueAt.equals("")) {
            row.set("due_at", dueAt);
        }
        return insert("activities", row, ctx.user_id(), input);
    }
}
