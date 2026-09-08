/*
 * pipeline_board — the kanban: one column per stage, each carrying its open
 * deals and their summed value.
 *
 * The second op HS-9 unblocked. `total_value` sums `opportunities.amount`
 * per stage, and the sum is what could not be spelled before
 * `GetKind::Float`.
 *
 * The nesting is a loop inside a loop over two lists ALREADY READ — both
 * crossings are outside both loops, which is what keeps the read count a
 * static quantity (`FL0090`/`FL0098`) rather than one per stage.
 */
package forge.crm;

import forge.Op;
import forge.OpContext;
import forge.Storage;
import forge.Value;

import java.util.List;

public final class PipelineBoard {

    public record BoardColumn(
        Value stage,
        List<Value> opportunities,
        double total_value,
        long count
    ) {}

    public record PipelineBoardOutput(List<BoardColumn> columns) {}

    /** One column. A `static` helper rather than a block-bodied lambda,
     * which the subset refuses (`FL2033`) — lambdas here are expressions. */
    private static BoardColumn column(Value stage, List<Value> opps) {
        String sid = stage.get("id", "");
        List<Value> in_stage = opps.stream()
            .filter(o -> o.get("stage_id", "").equals(sid))
            .toList();
        // `-0.0`, not `0.0`, and it is not a typo. Rust's
        // `Iterator::sum::<f64>()` folds from NEGATIVE zero — the identity
        // that is exact for every operand including `-0.0` itself — so the
        // Rust this replaces answered `-0.0` for a stage with no deals, and
        // serde writes that to the wire as `-0.0`. An accumulator seeded at
        // `0.0` agrees on every non-empty sum and parts on exactly the empty
        // one, which is what the CRM swap gate caught on `Closed Won`.
        double total = -0.0;
        for (Value o : in_stage) {
            total = total + o.get("amount", 0.0);
        }
        return new BoardColumn(stage, in_stage, total, in_stage.size());
    }

    @Op("pipeline_board")
    public static PipelineBoardOutput pipelineBoard(OpContext ctx, Value input) {
        List<Value> stages = Storage.rows(Storage.query(Value.obj(
            "from", "pipeline_stages",
            "sort", Value.list(Value.obj("column", "position", "direction", "asc")),
            "limit", 50L
        )));
        List<Value> opps = Storage.rows(Storage.query(Value.obj(
            "from", "opportunities",
            "filter", Value.obj("column", "status", "op", "eq", "value", "open"),
            "sort", Value.list(Value.obj("column", "close_date", "direction", "asc")),
            "follow", Value.list(Value.obj(
                "relationship", "account_id",
                "select", Value.list(Value.of("id"), Value.of("name"))
            )),
            "limit", 1000L
        )));

        return new PipelineBoardOutput(
            stages.stream().map(stage -> column(stage, opps)).toList()
        );
    }
}
