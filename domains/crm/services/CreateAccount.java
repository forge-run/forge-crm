/*
 * create_account — insert one account, tenant + owner stamped from the
 * trusted session.
 */
package forge.crm;

import forge.Forge;
import forge.Op;
import forge.OpContext;
import forge.OpError;
import forge.Storage;
import forge.Value;

import java.util.List;

public final class CreateAccount {

    public record CreateAccountOutput(Value id, long committed) {}

    /** `insert` — the shared write path, spelled per source because the
     * dialect compiles every file in isolation (FL2001: no shared helper
     * class). tenant_id arrives via `$auth:tenant_id`; owner_id is the
     * authenticated caller. */
    private static CreateAccountOutput insert(String table, Value row, String uid, Value input) {
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
        return new CreateAccountOutput(id, res.get("committed", 0L));
    }

    @Op("create_account")
    public static CreateAccountOutput createAccount(OpContext ctx, Value input) {
        String name = input.get("name", "");
        if (name.equals("")) {
            throw OpError.bad_request("name required");
        }
        Value row = Value.obj("name", name);
        String domain = input.get("domain", "");
        if (!domain.equals("")) {
            row.set("domain", domain);
        }
        String website = input.get("website", "");
        if (!website.equals("")) {
            row.set("website", website);
        }
        String industry = input.get("industry", "");
        if (!industry.equals("")) {
            row.set("industry", industry);
        }
        String type = input.get("type", "");
        if (!type.equals("")) {
            row.set("type", type);
        }
        String phone = input.get("phone", "");
        if (!phone.equals("")) {
            row.set("phone", phone);
        }
        return insert("accounts", row, ctx.user_id(), input);
    }
}
