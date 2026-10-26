import type * as React from 'react';

export type IconName = 'plus' | 'minus' | 'arrow-up-right' | 'arrow-down-left' | 'wallet' | 'credit-card' | 'calendar' | 'tag' | 'triangle-alert' | 'circle-check' | 'circle-alert' | 'info' | 'check' | 'x' | 'pencil' | 'trash-2' | 'search' | 'funnel' | 'chevron-down' | 'repeat' | 'house' | 'chart-pie' | 'settings' | 'log-out' | 'eye' | 'eye-off' | 'inbox';
export type MoneyKind = 'auto' | 'income' | 'expense' | 'neutral';

/** The TabWallet mark or full signature, coloured by theme tokens. */
export interface LogoProps { variant?: 'full' | 'mark'; height?: number; tone?: 'default' | 'white' | 'mono'; label?: string; className?: string }
export declare function Logo(props: LogoProps): React.ReactElement;

/** A Lucide outline icon (subset), stroke 1.75, colour = currentColor. Decorative unless `label` is set. */
export interface IconProps { name: IconName; size?: 16 | 20 | 24 | number; strokeWidth?: number; label?: string; className?: string }
export declare function Icon(props: IconProps): React.ReactElement;

export interface ButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: 'primary' | 'secondary' | 'ghost' | 'danger';
  size?: 'sm' | 'md' | 'lg';
  icon?: IconName; iconRight?: IconName; fullWidth?: boolean; loading?: boolean;
}
export declare function Button(props: ButtonProps): React.ReactElement;

export interface SegmentedOption { value: string; label: string }
export interface SegmentedControlProps { options: SegmentedOption[]; value?: string; defaultValue?: string; onChange?: (value: string) => void; label: string; size?: 'sm' | 'md'; className?: string }
export declare function SegmentedControl(props: SegmentedControlProps): React.ReactElement;

interface FieldProps { label?: string; hint?: string; error?: string; optional?: boolean; id?: string; className?: string }
export interface TextFieldProps extends FieldProps, Omit<React.InputHTMLAttributes<HTMLInputElement>, 'id' | 'className'> {}
export declare function TextField(props: TextFieldProps): React.ReactElement;
/** Text input with an R$ prefix; the value stays a string ("1.234,56"): parse it with your form library. */
export interface MoneyInputProps extends TextFieldProps {}
export declare function MoneyInput(props: MoneyInputProps): React.ReactElement;
export interface SelectProps extends FieldProps, Omit<React.SelectHTMLAttributes<HTMLSelectElement>, 'id' | 'className'> {
  options: { value: string; label: string; disabled?: boolean }[]; placeholderOption?: string;
}
export declare function Select(props: SelectProps): React.ReactElement;

/** A BRL amount: tabular figures; income gets "+ " and green, expense "− " and red. */
export interface MoneyProps { value: number; kind?: MoneyKind; size?: 'sm' | 'md' | 'lg'; showSign?: boolean; className?: string }
export declare function Money(props: MoneyProps): React.ReactElement;

export interface BadgeProps { tone?: 'neutral' | 'brand' | 'income' | 'expense' | 'warning'; icon?: IconName; children?: React.ReactNode; className?: string }
export declare function Badge(props: BadgeProps): React.ReactElement;

export interface TransactionRowProps {
  description: string; amount: number; kind?: 'income' | 'expense'; category?: string; date?: string;
  installment?: { current: number; total: number }; recurring?: boolean;
  onEdit?: () => void; onDelete?: () => void; as?: 'li' | 'div'; className?: string;
}
export declare function TransactionRow(props: TransactionRowProps): React.ReactElement;

export interface CardProps { title?: React.ReactNode; subtitle?: React.ReactNode; action?: React.ReactNode; flush?: boolean; as?: string; children?: React.ReactNode; className?: string }
export declare function Card(props: CardProps): React.ReactElement;

export interface DialogProps { open?: boolean; title: React.ReactNode; tone?: 'default' | 'danger'; children?: React.ReactNode; actions?: React.ReactNode; onClose?: () => void; inline?: boolean }
export declare function Dialog(props: DialogProps): React.ReactElement | null;

export interface EmptyStateProps { icon?: IconName; title: string; description?: string; action?: React.ReactNode; className?: string }
export declare function EmptyState(props: EmptyStateProps): React.ReactElement;

export interface AlertProps { tone?: 'info' | 'success' | 'warning' | 'danger'; title?: React.ReactNode; children?: React.ReactNode; action?: React.ReactNode; className?: string }
export declare function Alert(props: AlertProps): React.ReactElement;

export interface BalanceCardProps { balance: number; income?: number; expense?: number; period?: string; label?: string; className?: string }
export declare function BalanceCard(props: BalanceCardProps): React.ReactElement;

export interface BudgetBarProps { category: string; spent: number; limit: number; warnAt?: number; className?: string }
export declare function BudgetBar(props: BudgetBarProps): React.ReactElement;

export interface InvoiceCardProps {
  cardName: string; month?: string; total: number; status?: 'open' | 'closed' | 'paid';
  closingDay?: number; dueDate?: string; paid?: number; onAdvance?: () => void; className?: string;
}
export declare function InvoiceCard(props: InvoiceCardProps): React.ReactElement;

export interface NavItem { label: string; href?: string; icon?: IconName; active?: boolean }
export interface AppHeaderProps { items?: NavItem[]; right?: React.ReactNode; homeHref?: string; className?: string }
export declare function AppHeader(props: AppHeaderProps): React.ReactElement;

export declare function formatBRL(value: number): string;

declare global {
  interface Window {
    TabWallet: {
      Logo: typeof Logo; Icon: typeof Icon; Button: typeof Button; SegmentedControl: typeof SegmentedControl;
      TextField: typeof TextField; MoneyInput: typeof MoneyInput; Select: typeof Select; Money: typeof Money; Badge: typeof Badge;
      TransactionRow: typeof TransactionRow; Card: typeof Card; Dialog: typeof Dialog; EmptyState: typeof EmptyState; Alert: typeof Alert;
      BalanceCard: typeof BalanceCard; BudgetBar: typeof BudgetBar; InvoiceCard: typeof InvoiceCard; AppHeader: typeof AppHeader;
      formatBRL: typeof formatBRL;
    };
  }
}
