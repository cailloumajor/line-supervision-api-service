import "contrib/tomhollingworth/events"
import "dict"

__bucket__ = "testbucket"
__machine_set__ = ["1", "2"]
__machine_dict__ = ["1": "machine1", "2": "machine2"]

from(bucket: __bucket__)
    |> range(start: -24h)
    |> filter(fn: (r) => r._measurement == "dbLineSupervision.machine")
    |> filter(fn: (r) => r._field =~ /^machineState\./)
    |> filter(fn: (r) => contains(value: r.machine_index, set: __machine_set__))
    |> group(columns: ["machine_index"])
    |> pivot(rowKey: ["_time"], columnKey: ["_field"], valueColumn: "_value")
    |> map(
        fn: (r) => ({r with
            state_index: if r["machineState.cycle"] then
                3
            else if r["machineState.alarm"] then
                1
            else if r["machineState.alert"] then
                2
            else
                0,
        }),
    )
    |> drop(fn: (column) => column =~ /^machineState\./)
    |> aggregateWindow(
        every: 1m,
        column: "state_index",
        fn: (tables=<-, column) => tables |> min(column) |> sum(column),
    )
    |> fill(column: "state_index", value: -1)
    |> duplicate(column: "state_index", as: "_diff")
    |> difference(columns: ["_diff"], keepFirst: true)
    |> filter(fn: (r) => not exists r._diff or r._diff != 0)
    |> drop(columns: ["_diff"])
    |> events.duration(
        unit: 1s,
        columnName: "duration",
    )
    |> filter(fn: (r) => r.state_index != -1)
    |> map(
        fn: (r) => ({r with
            machine_name: dict.get(
                default: "not found",
                dict: __machine_dict__,
                key: r.machine_index,
            ),
        }),
    )
