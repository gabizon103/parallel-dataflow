import seaborn as sns
import matplotlib.pyplot as plt
import pandas as pd
import os
import argparse

# Group by pass & executor and calculate mean of all iterations.
# Show a bar plot of the mean values, grouped by pass and colored by executor.
def aggregate_bar(df, ycol, out_dir, limit=None):
    df = df.drop(columns=['name'])
    df = df.groupby(["executor", "pass"]).mean().reset_index()

    if limit:
        # Take only the top N executors for each pass
        df = df.groupby("pass").apply(lambda x: x.nsmallest(limit, ycol)).reset_index(drop=True)

    sequential_times = df[df["executor"] == "sequential"].set_index("pass")[ycol]
    df["speedup"] = df.apply(
        lambda row: sequential_times[row["pass"]] / row[ycol] if row["pass"] in sequential_times else None,
        axis=1
    )

    # Set the color palette
    palette = sns.color_palette("husl", len(df["executor"].unique()))
    sns.set_palette(palette)
    # Create a bar plot
    plt.figure(figsize=(10, 6))
    ax = sns.barplot(x="pass", y=ycol, hue="executor", data=df)

    for bar, (yval, speedup) in zip(ax.patches, zip(df[ycol], df["speedup"])):

        label = f"{speedup:.2f}x" if pd.notna(speedup) else ""
        # Place the label above the bar
        ax.text(
            bar.get_x() + bar.get_width() / 2,  # Horizontal alignment
            bar.get_height() + (0.05 * ax.get_ylim()[1]),  # Slightly above the bar
            label,
            ha="center",
            va="bottom",
            rotation=45,
            fontsize=9,
            color="black"
        )

    plt.title(f"{ycol.title()} by Pass and Executor")
    plt.xlabel("Pass")
    plt.ylabel("Time (ns)")
    plt.ylim(bottom=0)
    plt.legend(title="Executor")
    plt.tight_layout()
    if limit:
        plt.savefig(f"{out_dir}/averages_{ycol}_top_{limit}.png")
    else:
        plt.savefig(f"{out_dir}/averages_{ycol}.png")

# Group by executor and name and calculate mean of all iterations for a specific pass
# Show bar plot of mean values, grouped by benchmark name and colored by executor
def aggregate_bar_by_benchmark(df, ycol, out_dir, pass_, limit=None):
    df = df[df["pass"] == pass_]
    df = df.drop(columns=["pass"])

    df = df.groupby(["executor", "name"]).mean().reset_index()
    if limit:
        # Take only the top N executors for each pass
        df = df.groupby("pass").apply(lambda x: x.nsmallest(limit, ycol)).reset_index(drop=True)

    sequential_times = df[df["executor"] == "sequential"].set_index("name")[ycol]
    df["speedup"] = df.apply(
        lambda row: sequential_times[row["name"]] / row[ycol] if row["name"] in sequential_times else None,
        axis=1
    )

    # Set the color palette
    palette = sns.color_palette(n_colors=len(df["executor"].unique()))
    sns.set_palette(palette)
    # Create a bar plot
    plt.figure(figsize=(10, 6))
    ax = sns.barplot(x="name", y=ycol, hue="executor", data=df)
    
    for bar, (yval, speedup) in zip(ax.patches, zip(df[ycol], df["speedup"])):
        label = f"{speedup:.2f}x" if pd.notna(speedup) else ""
        ax.text(
            bar.get_x() + bar.get_width() / 2,
            yval,
            label,
            ha="center",
            va="bottom",
            rotation=90,
            fontsize=7,
            color="black"
        )

    plt.title(f"{ycol.title()} for {pass_} by Benchmark and Executor")
    plt.xlabel("Benchmark")
    plt.ylabel("Time (ns)")
    plt.ylim(bottom=0)
    plt.legend(title="Executor")
    plt.tight_layout()
    if limit:
        plt.savefig(f"{out_dir}/averages_{ycol}_top_{limit}.png")
    else:
        plt.savefig(f"{out_dir}/averages_by_bmark_{pass_}_{ycol}.png")

# Create a violin plot of the data, grouped by pass and colored by executor.
def violin(df, ycol, out_dir, limit=None):
    df = df.drop(columns=['name'])
    
    if limit:
        # Take only the top N executors with the smallest mean values
        top = df.groupby(["pass", "executor"]).mean().reset_index()
        top = top.groupby("pass").apply(lambda x: x.nsmallest(limit, ycol)).reset_index(drop=True)
        # Keep rows where the executor and pass pair is in top
        df = df[df.set_index(["pass", "executor"]).index.isin(top.set_index(["pass", "executor"]).index)]

    # Set the color palette
    palette = sns.color_palette("husl", len(df["executor"].unique()))
    sns.set_palette(palette)

    # Create a violin plot
    plt.figure(figsize=(10, 6))
    sns.violinplot(x="pass", y=ycol, hue="executor", data=df)
    plt.title(f"{ycol.title()} by Pass and Executor")
    plt.xlabel("Pass")
    plt.ylabel("Time (ns)")
    plt.ylim(bottom=0)
    plt.legend(title="Executor")
    plt.tight_layout()
    if limit:
        plt.savefig(f"{out_dir}/violin_{ycol}_top_{limit}.png")
    else:
        plt.savefig(f"{out_dir}/violin_{ycol}.png")

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Plot graphs from performance data.")
    parser.add_argument("--data", type=str, help="Path to the CSV file", default="perf.csv")
    parser.add_argument("--out_dir", type=str, help="Output directory for the plots", default="results")
    args = parser.parse_args()

    # Read the CSV file
    if not os.path.exists(args.data):
        raise FileNotFoundError(f"The file {args.data} does not exist.")
    
    os.makedirs(args.out_dir, exist_ok=True)
    df = pd.read_csv(args.data)

    aggregate_bar(df, "runtime", args.out_dir)
    aggregate_bar(df, "loadtime", args.out_dir)
    aggregate_bar(df, "runtime", args.out_dir, limit=3)
    aggregate_bar(df, "loadtime", args.out_dir, limit=3)
    aggregate_bar_by_benchmark(df, "runtime", args.out_dir, "ReachingDefinitions")

    violin(df, "runtime", args.out_dir)
    violin(df, "loadtime", args.out_dir)
    violin(df, "runtime", args.out_dir, limit=3)
    violin(df, "loadtime", args.out_dir, limit=3)