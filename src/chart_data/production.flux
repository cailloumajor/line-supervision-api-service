import "date"

__bucket__ = "testbucket"
__start__ = -8h
__machine_set__ = ["2", "3"]
__total_function__ = (r) => r["2"] + r["3"]
offset = duration(v: "${date.minute(t: __start__)}m")
total_function = __total_function__

from(bucket: __bucket__)
    |> range(start: __start__)
    |> filter(fn: (r) => r._measurement == "dbLineSupervision.machine")
    |> filter(fn: (r) => r._field == "counters.production")
    |> filter(fn: (r) => contains(value: r.machine_index, set: __machine_set__))
    |> pivot(columnKey: ["machine_index"], rowKey: ["_time"], valueColumn: "_value")
    |> window(every: 1h, offset: offset)
    |> increase(columns: __machine_set__)
    |> top(n: 1, columns: ["_time"])
    |> map(fn: (r) => ({r with total: total_function(r)}))
