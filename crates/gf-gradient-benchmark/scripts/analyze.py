#!/usr/bin/env python3
"""
Analysis script for gradient benchmark results.

Generates:
- Convergence curves (12 formats overlaid per surface)
- Grad norm distribution by format
- Format ranking table
"""

import pandas as pd
import matplotlib.pyplot as plt
import numpy as np
from pathlib import Path
import sys

# Directory containing CSV files
RESULTS_DIR = Path(__file__).parent.parent / "results"
OUTPUT_DIR = RESULTS_DIR / "plots"


def load_all_results(results_dir: Path) -> pd.DataFrame:
    """Load all CSV files from benchmark runs."""
    data = []
    for csv_path in results_dir.glob("*.csv"):
        # Parse filename: gradient_metrics_{format}_{surface}_{seed}.csv
        stem = csv_path.stem
        parts = stem.split('_')
        if len(parts) >= 3:
            format_name = parts[0]
            surface_name = parts[1]
            seed = int(parts[2])

            df = pd.read_csv(csv_path)
            df['format'] = format_name
            df['surface'] = surface_name
            df['seed'] = seed
            data.append(df)

    return pd.concat(data, ignore_index=True)


def plot_convergence_curves(all_data: pd.DataFrame, output_dir: Path):
    """Plot convergence curves: 12 formats overlaid per surface."""
    surfaces = sorted(all_data['surface'].unique())

    for surface in surfaces:
        plt.figure(figsize=(14, 10))
        formats = sorted(all_data[all_data['surface'] == surface]['format'].unique())

        for fmt in formats:
            subset = all_data[
                (all_data['surface'] == surface) &
                (all_data['format'] == fmt)
            ]
            # Average across seeds
            avg = subset.groupby('step').agg({
                'loss': 'mean',
                'grad_norm': 'mean'
            }).reset_index()

            plt.plot(avg['step'], avg['loss'], label=fmt, alpha=0.7, linewidth=1.5)

        plt.xlabel('Step')
        plt.ylabel('Loss')
        plt.title(f'Convergence: {surface}')
        plt.legend(fontsize=9)
        plt.grid(True, alpha=0.3)
        plt.tight_layout()
        plt.savefig(output_dir / f'convergence_{surface}.png', dpi=300)
        plt.close()


def plot_grad_norm_by_format(all_data: pd.DataFrame, output_dir: Path):
    """Boxplot of gradient norms by format (final step)."""
    plt.figure(figsize=(14, 8))

    # Get final gradient norms
    final_grads = all_data[all_data['step'] == all_data['step'].max()]

    # Order formats by average grad norm (ascending)
    formats_order = final_grads.groupby('format')['grad_norm'].mean().sort_values().index.tolist()

    data_for_plot = [
        final_grads[final_grads['format'] == f]['grad_norm'].values
        for f in formats_order
    ]

    plt.boxplot(data_for_plot, labels=formats_order)
    plt.xticks(rotation=45, ha='right')
    plt.ylabel('Final Gradient Norm')
    plt.title('Gradient Norm by Format (lower = more stable)')
    plt.grid(True, axis='y', alpha=0.3)
    plt.tight_layout()
    plt.savefig(output_dir / 'grad_norm_by_format.png', dpi=300)
    plt.close()


def plot_snr_by_format(all_data: pd.DataFrame, output_dir: Path):
    """Bar chart of SNR by format (higher = better)."""
    plt.figure(figsize=(14, 8))

    # Average SNR across all data
    avg_snr = all_data.groupby('format')['snr'].mean().sort_values(ascending=True)

    formats = avg_snr.index.tolist()
    values = avg_snr.values.tolist()

    colors = ['#2ecc71' if fmt.startswith('GF') else '#1f77b4' for fmt in formats]

    plt.barh(formats, values, color=colors)
    plt.xlabel('Signal-to-Noise Ratio (higher = better)')
    plt.ylabel('SNR')
    plt.title('SNR by Format')
    plt.grid(True, axis='x', alpha=0.3)
    plt.tight_layout()
    plt.savefig(output_dir / 'snr_by_format.png', dpi=300)
    plt.close()


def plot_bias_vs_fp32_heatmap(all_data: pd.DataFrame, output_dir: Path):
    """Heatmap of bias vs FP32 (format × surface)."""
    plt.figure(figsize=(16, 10))

    # Get final bias values
    final_data = all_data[all_data['step'] == all_data['step'].max()]

    # Pivot: format × surface, value = bias_vs_fp32
    pivot = final_data.pivot_table(
        index='format',
        columns='surface',
        values='bias_vs_fp32'
    )

    # Order formats and surfaces
    formats = sorted(final_data['format'].unique())
    surfaces = sorted(final_data['surface'].unique())

    pivot = pivot.reindex(index=formats, columns=surfaces)

    # Create heatmap
    im = plt.imshow(
        pivot.values,
        cmap='RdYlGn_r',  # Blue = low bias, Red = high bias
        aspect='auto',
        vmin=0,
        vmax=0.5  # Clamp at 0.5
    )

    plt.colorbar(im, label='Bias vs FP32')
    plt.xticks(np.arange(len(formats)), formats, rotation=45, ha='right')
    plt.yticks(np.arange(len(surfaces)), surfaces, rotation=0, ha='right')
    plt.xlabel('Format')
    plt.ylabel('Surface')
    plt.title('Bias vs FP32 Heatmap (lower = more accurate)')
    plt.tight_layout()
    plt.savefig(output_dir / 'bias_heatmap.png', dpi=300)
    plt.close()


def generate_format_ranking(all_data: pd.DataFrame, output_dir: Path):
    """Generate markdown table ranking formats."""
    # Get final metrics
    final_data = all_data[all_data['step'] == all_data['step'].max()]

    # Group by format and surface, then average across seeds
    ranking = final_data.groupby(['format', 'surface']).agg({
        'loss': 'mean',
        'grad_norm': 'mean',
        'snr': 'mean',
        'bias_vs_fp32': 'mean'
    }).reset_index()

    # Sort by loss (primary metric)
    ranking = ranking.sort_values('loss')

    with open(output_dir / 'format_ranking.md', 'w') as f:
        f.write('# Format Ranking (final loss, averaged across surfaces and seeds)\n\n')
        f.write('| Format | Surface | Loss | Grad Norm | SNR | Bias vs FP32 |\n')
        f.write('|--------|---------|------|----------|-----|-------------|\n')

        for (fmt, surface), row in ranking.iterrows():
            f.write(f'| {fmt:<8} | {surface:<12} | {row["loss"]:.6f} | '
                    f'{row["grad_norm"]:.6f} | {row["snr"]:.6f} | {row["bias_vs_fp32"]:.6f} |\n')

    print(f"Format ranking table written to {output_dir / 'format_ranking.md'}")


def print_summary(all_data: pd.DataFrame):
    """Print summary statistics."""
    final_data = all_data[all_data['step'] == all_data['step'].max()]

    print("\n=== Summary Statistics ===")
    print(f"Total experiments: {len(all_data)}")
    print(f"Formats: {sorted(all_data['format'].unique())}")
    print(f"Surfaces: {sorted(all_data['surface'].unique())}")
    print(f"Seeds: {sorted(all_data['seed'].unique())}")

    print("\n=== By Format (final loss) ===")
    for fmt in sorted(all_data['format'].unique()):
        subset = final_data[final_data['format'] == fmt]
        print(f"{fmt:<8}: Loss = {subset['loss'].mean():.6f} ± {subset['loss'].std():.6f}")


def main():
    if len(sys.argv) > 1:
        results_dir = Path(sys.argv[1])
    else:
        results_dir = RESULTS_DIR

    output_dir = results_dir / "plots"
    output_dir.mkdir(parents=True, exist_ok=True)

    print(f"Loading results from {results_dir}")
    all_data = load_all_results(results_dir)

    if all_data.empty:
        print("No CSV files found!")
        sys.exit(1)

    print(f"Loaded {len(all_data)} experiment rows")

    # Generate all plots
    print("Generating convergence curves...")
    plot_convergence_curves(all_data, output_dir)

    print("Generating grad norm distribution...")
    plot_grad_norm_by_format(all_data, output_dir)

    print("Generating SNR chart...")
    plot_snr_by_format(all_data, output_dir)

    print("Generating bias heatmap...")
    plot_bias_vs_fp32_heatmap(all_data, output_dir)

    print("Generating format ranking table...")
    generate_format_ranking(all_data, output_dir)

    # Print summary
    print_summary(all_data)

    print(f"\n=== Analysis Complete ===")
    print(f"Plots saved to {output_dir}")
    print(f"Ranking table: {output_dir / 'format_ranking.md'}")


if __name__ == '__main__':
    main()
