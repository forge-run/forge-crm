/*
 * update_account — apply a client patch to one accounts row by id, RLS-scoped to the
 * caller's tenant.
 *
 * Protected columns (tenant_id, id, audit) are stripped so a patch can never
 * re-tenant a row or forge its identity. The Rust op REMOVED those keys from
 * the client's object; the dialect has no key removal, so the same result is
 * built by copying the survivors in key order — which is the order `remove`
 * left behind.
 */
package forge.crm;

import forge.Op;
import forge.OpContext;
import forge.OpError;
import forge.Storage;
import forge.Value;

import java.util.List;

public final class UpdateAccount {

    public record UpdateAccountOutput(long committed) {}

    @Op("update_account")
    public static UpdateAccountOutput updateAccount(OpContext ctx, Value input) {
        String id = input.get("id", "");
        if (id.equals("")) {
            throw OpError.bad_request("id required");
        }
        Value given = input.get("patch", Value.obj());
        Value patch = Value.obj();
        for (String k : given.keys()) {
            if (!List.of("id", "tenant_id", "created_at", "created_by",
                         "last_modified_at", "last_modified_by").contains(k)) {
                patch.set(k, given.getItem(k));
            }
        }
        if (patch.keys().isEmpty()) {
            throw OpError.bad_request("empty patch");
        }
        Value res = Storage.mutate(Value.obj(
            "table", "accounts",
            "op", "update",
            "filter", Value.obj("column_equals", Value.obj("column", "id", "value", id)),
            "patch", patch
        ));
        return new UpdateAccountOutput(res.get("committed", 0L));
    }
}
