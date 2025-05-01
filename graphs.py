import seaborn as sns
import matplotlib.pyplot as plt
import pandas as pd
import os
import argparse

# Group by pass & executor and calculate mean of all iterations.
# Show a bar plot of the mean values, grouped by pass and colored by executor.
def aggregate_bar(df, ycol, out_dir):
    df = df.groupby(["pass", "executor"]).mean().reset_index()
    # Set the color palette
    palette = sns.color_palette("husl", len(df["executor"].unique()))
    sns.set_palette(palette)
    # Create a bar plot
    plt.figure(figsize=(10, 6))
    sns.barplot(x="pass", y=ycol, hue="executor", data=df)
    plt.title(f"{ycol.title()} by Pass and Executor")
    plt.xlabel("Pass")
    plt.ylabel("Time (ns)")
    plt.legend(title="Executor")
    plt.tight_layout()
    plt.savefig(f"{out_dir}/averages_{ycol}.png")

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