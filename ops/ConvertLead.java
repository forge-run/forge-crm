/*
 * convert_lead — turn a qualified lead into a contact (and optionally an
 * opportunity), then mark the lead converted. A small multi-step write —
 * the kind of thing that justifies an op over the raw auto-CRUD API.
 */
package forge.crm;

import forge.Forge;
import forge.Op;
import forge.OpContext;
import forge.OpError;
import forge.Storage;
import forge.Value;

import java.util.List;

public final class ConvertLead {

    public record Inserted(Value id, long committed) {}

    public record ConvertLeadOutput(Value contact_id, String account_id, Value opportunity_id) {}

    private static Inserted insert(String table, Value row, String uid, Value input) {
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
        return new Inserted(id, res.get("committed", 0L));
    }

    @Op("convert_lead")
    public static ConvertLeadOutput convertLead(OpContext ctx, Value input) {
        String id = input.get("id", "");
        if (id.equals("")) {
            throw OpError.bad_request("id required");
        }
        List<Value> leadRows = Storage.rows(Storage.query(Value.obj(
            "from", "leads",
            "filter", Value.obj("column", "id", "op", "eq", "value", id),
            "limit", 1L
        )));
        if (leadRows.isEmpty()) {
            throw OpError.not_found();
        }
        Value lead = Forge.nth(leadRows, 0L);
        String uid = ctx.user_id();

        // 1. contact from the lead's person fields
        Value contactRow = Value.obj(
            "first_name", lead.get("first_name", ""),
            "last_name", lead.get("last_name", ""),
            "email", lead.get("email", ""),
            "phone", lead.get("phone", ""),
            "title", lead.get("title", "")
        );
        Inserted contact = insert("contacts", contactRow, uid, input);
        Value contactId = contact.id();

        // 2. optional opportunity
        Value opportunityId = Value.of("");
        if (input.get("create_opportunity", false)) {
            String company = lead.get("company", "");
            String name = company + " opportunity";
            if (company.equals("")) {
                name = lead.get("first_name", "") + " opportunity";
            }
            Inserted opportunity =
                insert("opportunities", Value.obj("name", name, "status", "open"), uid, input);
            opportunityId = opportunity.id();
        }

        // 3. mark the lead converted
        Value marked = Storage.mutate(Value.obj(
            "table", "leads",
            "op", "update",
            "filter", Value.obj("column_equals", Value.obj("column", "id", "value", id)),
            "patch", Value.obj(
                "converted", true,
                "converted_contact_id", contactId,
                "status", "qualified"
            )
        ));
        return new ConvertLeadOutput(contactId, "", opportunityId);
    }
}
