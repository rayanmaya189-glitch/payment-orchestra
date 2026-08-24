# 09 — Shared Components

## 1. Design System

### Color Tokens

```typescript
// Tailwind CSS custom colors
const colors = {
  // Status colors
  success: '#22c55e',   // green-500
  warning: '#eab308',   // yellow-500
  error: '#ef4444',     // red-500
  info: '#3b82f6',      // blue-500

  // Payment status
  created: '#6b7280',   // gray-500
  authorizing: '#3b82f6', // blue-500
  authorized: '#22c55e', // green-500
  captured: '#22c55e',  // green-500
  failed: '#ef4444',    // red-500
  voided: '#6b7280',    // gray-500
  refunded: '#f97316',  // orange-500

  // AML severity
  aml_low: '#3b82f6',
  aml_medium: '#eab308',
  aml_high: '#f97316',
  aml_critical: '#ef4444',
};
```

---

## 2. Status Badge

```tsx
interface StatusBadgeProps {
  status: string;
  variant?: 'default' | 'success' | 'warning' | 'error' | 'info';
  pulse?: boolean;
}

function StatusBadge({ status, variant, pulse }: StatusBadgeProps) {
  return (
    <span className={cn(
      'inline-flex items-center rounded-full px-2.5 py-0.5 text-xs font-medium',
      variant === 'success' && 'bg-green-100 text-green-800',
      variant === 'warning' && 'bg-yellow-100 text-yellow-800',
      variant === 'error' && 'bg-red-100 text-red-800',
      variant === 'info' && 'bg-blue-100 text-blue-800',
      pulse && 'animate-pulse',
    )}>
      {pulse && <span className="mr-1 h-2 w-2 rounded-full bg-current" />}
      {status}
    </span>
  );
}
```

---

## 3. Data Table

```tsx
interface DataTableProps<T> {
  data: T[];
  columns: ColumnDef<T>[];
  loading?: boolean;
  pagination?: { cursor: string; hasMore: boolean };
  onLoadMore?: () => void;
  onSort?: (column: string, direction: 'asc' | 'desc') => void;
  onRowClick?: (row: T) => void;
  emptyMessage?: string;
}

function DataTable<T>({ data, columns, loading, pagination, ...props }: DataTableProps<T>) {
  const table = useReactTable({
    data,
    columns,
    getCoreRowModel: getCoreRowModel(),
  });

  return (
    <div className="rounded-md border">
      <Table>
        <TableHeader>
          {table.getHeaderGroups().map(headerGroup => (
            <TableRow key={headerGroup.id}>
              {headerGroup.headers.map(header => (
                <TableHead key={header.id}>
                  {flexRender(header.column.columnDef.header, header.getContext())}
                </TableHead>
              ))}
            </TableRow>
          ))}
        </TableHeader>
        <TableBody>
          {loading ? (
            <TableRow>
              <TableCell colSpan={columns.length} className="h-24 text-center">
                <LoadingSpinner />
              </TableCell>
            </TableRow>
          ) : table.getRowModel().rows.length === 0 ? (
            <TableRow>
              <TableCell colSpan={columns.length} className="h-24 text-center">
                {props.emptyMessage || 'No data'}
              </TableCell>
            </TableRow>
          ) : (
            table.getRowModel().rows.map(row => (
              <TableRow
                key={row.id}
                onClick={() => props.onRowClick?.(row.original)}
                className={props.onRowClick && 'cursor-pointer hover:bg-muted/50'}
              >
                {row.getVisibleCells().map(cell => (
                  <TableCell key={cell.id}>
                    {flexRender(cell.column.columnDef.cell, cell.getContext())}
                  </TableCell>
                ))}
              </TableRow>
            ))
          )}
        </TableBody>
      </Table>
      {pagination?.hasMore && (
        <div className="flex justify-center p-4">
          <Button onClick={props.onLoadMore} disabled={loading}>
            Load More
          </Button>
        </div>
      )}
    </div>
  );
}
```

---

## 4. Money Display

```tsx
function MoneyDisplay({ amount, currency }: { amount: number; currency: string }) {
  const formatted = new Intl.NumberFormat('en-AE', {
    style: 'currency',
    currency: currency,
    minimumFractionDigits: getCurrencyPrecision(currency),
  }).format(amount / Math.pow(10, getCurrencyPrecision(currency)));

  return <span className="font-mono">{formatted}</span>;
}

function getCurrencyPrecision(currency: string): number {
  switch (currency) {
    case 'BHD':
    case 'KWD':
      return 3;
    case 'JPY':
      return 0;
    default:
      return 2;
  }
}
```

---

## 5. Confirmation Dialog

```tsx
interface ConfirmDialogProps {
  title: string;
  description: string;
  confirmLabel: string;
  confirmVariant?: 'default' | 'destructive';
  onConfirm: () => Promise<void>;
  loading?: boolean;
}

function ConfirmDialog({ title, description, confirmLabel, ...props }: ConfirmDialogProps) {
  return (
    <AlertDialog>
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>{title}</AlertDialogTitle>
          <AlertDialogDescription>{description}</AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel>Cancel</AlertDialogCancel>
          <AlertDialogAction
            onClick={props.onConfirm}
            disabled={props.loading}
            className={props.confirmVariant === 'destructive' && 'bg-red-600 hover:bg-red-700'}
          >
            {props.loading ? <LoadingSpinner /> : confirmLabel}
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}
```

---

## 6. Form Components

### Amount Input

```tsx
function AmountInput({ value, currency, onChange, max }: AmountInputProps) {
  return (
    <div className="flex items-center gap-2">
      <Input
        type="number"
        value={value}
        onChange={(e) => onChange(parseFloat(e.target.value))}
        min={0}
        max={max}
        step={0.01}
      />
      <Select value={currency} onValueChange={(val) => onChange(undefined, val)}>
        <SelectTrigger className="w-24">
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          <SelectItem value="AED">AED</SelectItem>
          <SelectItem value="USD">USD</SelectItem>
          <SelectItem value="EUR">EUR</SelectItem>
          <SelectItem value="SAR">SAR</SelectItem>
        </SelectContent>
      </Select>
    </div>
  );
}
```

### Date Range Picker

```tsx
function DateRangePicker({ value, onChange }: DateRangePickerProps) {
  return (
    <div className="flex items-center gap-2">
      <Popover>
        <PopoverTrigger asChild>
          <Button variant="outline">
            <CalendarIcon className="mr-2 h-4 w-4" />
            {value ? format(value.start, 'MMM d') + ' - ' + format(value.end, 'MMM d') : 'Select dates'}
          </Button>
        </PopoverTrigger>
        <PopoverContent className="w-auto p-0">
          <Calendar
            mode="range"
            selected={value}
            onSelect={onChange}
          />
        </PopoverContent>
      </Popover>
    </div>
  );
}
```

---

## 7. Loading States

```tsx
// Skeleton loading for tables
function TableSkeleton({ rows = 5, columns = 5 }) {
  return (
    <div className="space-y-3">
      {Array.from({ length: rows }).map((_, i) => (
        <div key={i} className="flex gap-4">
          {Array.from({ length: columns }).map((_, j) => (
            <Skeleton key={j} className="h-4 flex-1" />
          ))}
        </div>
      ))}
    </div>
  );
}

// Full page loading
function PageLoading() {
  return (
    <div className="flex h-96 items-center justify-center">
      <LoadingSpinner size="lg" />
    </div>
  );
}
```
