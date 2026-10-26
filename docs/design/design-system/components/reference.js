/* @ds-bundle: {"format": 4, "namespace": "TabWallet", "components": [{"name": "Logo"}, {"name": "Icon"}, {"name": "Button"}, {"name": "SegmentedControl"}, {"name": "TextField"}, {"name": "MoneyInput"}, {"name": "Select"}, {"name": "Money"}, {"name": "Badge"}, {"name": "TransactionRow"}, {"name": "Card"}, {"name": "Dialog"}, {"name": "EmptyState"}, {"name": "Alert"}, {"name": "BalanceCard"}, {"name": "BudgetBar"}, {"name": "InvoiceCard"}, {"name": "AppHeader"}]} */
(function () {
  var React = window.React, h = React.createElement;
  function cx() { return Array.prototype.filter.call(arguments, Boolean).join(' '); }
  function omit(o, keys) { var r = {}; for (var k in o) if (Object.prototype.hasOwnProperty.call(o, k) && keys.indexOf(k) < 0) r[k] = o[k]; return r; }
  var useId = React.useId || function () { var r = React.useRef('tw-' + Math.random().toString(36).slice(2, 8)); return r.current; };

  /* ---------- Icon (Lucide subset, ISC) ---------- */
  var ICONS = {"plus": "<path d=\"M5 12h14\"/> <path d=\"M12 5v14\"/>", "minus": "<path d=\"M5 12h14\"/>", "arrow-up-right": "<path d=\"M7 7h10v10\"/> <path d=\"M7 17 17 7\"/>", "arrow-down-left": "<path d=\"M17 7 7 17\"/> <path d=\"M17 17H7V7\"/>", "wallet": "<path d=\"M19 7V4a1 1 0 0 0-1-1H5a2 2 0 0 0 0 4h15a1 1 0 0 1 1 1v4h-3a2 2 0 0 0 0 4h3a1 1 0 0 0 1-1v-2a1 1 0 0 0-1-1\"/> <path d=\"M3 5v14a2 2 0 0 0 2 2h15a1 1 0 0 0 1-1v-4\"/>", "credit-card": "<rect width=\"20\" height=\"14\" x=\"2\" y=\"5\" rx=\"2\"/> <line x1=\"2\" x2=\"22\" y1=\"10\" y2=\"10\"/> <path d=\"M6 14h2\"/>", "calendar": "<path d=\"M8 2v3\"/> <path d=\"M16 2v3\"/> <rect x=\"3\" y=\"3\" width=\"18\" height=\"18\" rx=\"2\"/> <path d=\"M3 9h18\"/>", "tag": "<path d=\"M12.586 2.586A2 2 0 0 0 11.172 2H4a2 2 0 0 0-2 2v7.172a2 2 0 0 0 .586 1.414l8.704 8.704a2.426 2.426 0 0 0 3.42 0l6.58-6.58a2.426 2.426 0 0 0 0-3.42z\"/> <circle cx=\"7.5\" cy=\"7.5\" r=\".5\" fill=\"currentColor\"/>", "triangle-alert": "<path d=\"m21.73 18-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3\"/> <path d=\"M12 9v4\"/> <path d=\"M12 17h.01\"/>", "circle-check": "<circle cx=\"12\" cy=\"12\" r=\"10\"/> <path d=\"m16 9-5.5 5.5L8 12\"/>", "circle-alert": "<circle cx=\"12\" cy=\"12\" r=\"10\"/> <line x1=\"12\" x2=\"12\" y1=\"8\" y2=\"12\"/> <line x1=\"12\" x2=\"12.01\" y1=\"16\" y2=\"16\"/>", "info": "<circle cx=\"12\" cy=\"12\" r=\"10\"/> <path d=\"M12 16v-4\"/> <path d=\"M12 8h.01\"/>", "check": "<path d=\"M20 6 9 17l-5-5\"/>", "x": "<path d=\"M18 6 6 18\"/> <path d=\"m6 6 12 12\"/>", "pencil": "<path d=\"M21.174 6.812a1 1 0 0 0-3.986-3.987L3.842 16.174a2 2 0 0 0-.5.83l-1.321 4.352a.5.5 0 0 0 .623.622l4.353-1.32a2 2 0 0 0 .83-.497z\"/> <path d=\"m15 5 4 4\"/>", "trash-2": "<path d=\"M10 11v6\"/> <path d=\"M14 11v6\"/> <path d=\"M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6\"/> <path d=\"M3 6h18\"/> <path d=\"M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2\"/>", "search": "<path d=\"m21 21-4.34-4.34\"/> <circle cx=\"11\" cy=\"11\" r=\"8\"/>", "funnel": "<path d=\"M10 20a1 1 0 0 0 .553.895l2 1A1 1 0 0 0 14 21v-7a2 2 0 0 1 .517-1.341L21.74 4.67A1 1 0 0 0 21 3H3a1 1 0 0 0-.742 1.67l7.225 7.989A2 2 0 0 1 10 14z\"/>", "chevron-down": "<path d=\"m6 9 6 6 6-6\"/>", "repeat": "<path d=\"m17 2 4 4-4 4\"/> <path d=\"M3 11v-1a4 4 0 0 1 4-4h14\"/> <path d=\"m7 22-4-4 4-4\"/> <path d=\"M21 13v1a4 4 0 0 1-4 4H3\"/>", "house": "<path d=\"M15 21v-8a1 1 0 0 0-1-1h-4a1 1 0 0 0-1 1v8\"/> <path d=\"M3 10a2 2 0 0 1 .709-1.528l7-6a2 2 0 0 1 2.582 0l7 6A2 2 0 0 1 21 10v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z\"/>", "chart-pie": "<path d=\"M21 12c.552 0 1.005-.449.95-.998a10 10 0 0 0-8.953-8.951c-.55-.055-.998.398-.998.95v8a1 1 0 0 0 1 1z\"/> <path d=\"M21.21 15.89A10 10 0 1 1 8 2.83\"/>", "settings": "<path d=\"M9.671 4.136a2.34 2.34 0 0 1 4.659 0 2.34 2.34 0 0 0 3.319 1.915 2.34 2.34 0 0 1 2.33 4.033 2.34 2.34 0 0 0 0 3.831 2.34 2.34 0 0 1-2.33 4.033 2.34 2.34 0 0 0-3.319 1.915 2.34 2.34 0 0 1-4.659 0 2.34 2.34 0 0 0-3.32-1.915 2.34 2.34 0 0 1-2.33-4.033 2.34 2.34 0 0 0 0-3.831A2.34 2.34 0 0 1 6.35 6.051a2.34 2.34 0 0 0 3.319-1.915\"/> <circle cx=\"12\" cy=\"12\" r=\"3\"/>", "log-out": "<path d=\"m16 17 5-5-5-5\"/> <path d=\"M21 12H9\"/> <path d=\"M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4\"/>", "eye": "<path d=\"M2.062 12.348a1 1 0 0 1 0-.696 10.75 10.75 0 0 1 19.876 0 1 1 0 0 1 0 .696 10.75 10.75 0 0 1-19.876 0\"/> <circle cx=\"12\" cy=\"12\" r=\"3\"/>", "eye-off": "<path d=\"M10.733 5.076a10.744 10.744 0 0 1 11.205 6.575 1 1 0 0 1 0 .696 10.747 10.747 0 0 1-1.444 2.49\"/> <path d=\"M14.084 14.158a3 3 0 0 1-4.242-4.242\"/> <path d=\"M17.479 17.499a10.75 10.75 0 0 1-15.417-5.151 1 1 0 0 1 0-.696 10.75 10.75 0 0 1 4.446-5.143\"/> <path d=\"m2 2 20 20\"/>", "inbox": "<polyline points=\"22 12 16 12 14 15 10 15 8 12 2 12\"/> <path d=\"M5.45 5.11 2 12v6a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2v-6l-3.45-6.89A2 2 0 0 0 16.76 4H7.24a2 2 0 0 0-1.79 1.11z\"/>"};
  function Icon(p) {
    var size = p.size || 20;
    return h('svg', {
      className: cx('tw-icon', p.className), width: size, height: size, viewBox: '0 0 24 24', fill: 'none',
      stroke: 'currentColor', strokeWidth: p.strokeWidth || 1.75, strokeLinecap: 'round', strokeLinejoin: 'round',
      role: p.label ? 'img' : undefined, 'aria-label': p.label || undefined, 'aria-hidden': p.label ? undefined : true,
      focusable: 'false', dangerouslySetInnerHTML: { __html: ICONS[p.name] || '' }
    });
  }
  Icon.names = Object.keys(ICONS);

  /* ---------- money formatting ---------- */
  var BRL = new Intl.NumberFormat('pt-BR', { style: 'currency', currency: 'BRL' });
  var MINUS = '−', NBSP = ' ';
  function formatBRL(v) { return BRL.format(Math.abs(Number(v) || 0)).replace(/\s/g, NBSP); }
  function signedBRL(v, kind) {
    var n = Number(v) || 0;
    var k = kind && kind !== 'auto' ? kind : (n > 0 ? 'income' : n < 0 ? 'expense' : 'neutral');
    var sign = k === 'income' ? '+' + NBSP : k === 'expense' ? MINUS + NBSP : (n < 0 ? MINUS + NBSP : '');
    return { text: sign + formatBRL(n), kind: k };
  }

  /* ---------- Logo ---------- */
  var LOGO_W = 321, WT = 'translate(77.49 51.43) scale(0.0256)', GT = 'M474 0V-1186H20V-1440H1200V-1186H746V0ZM1600.0 30Q1484.0 30 1403.5 -14.5Q1323.0 -59 1281.5 -133.5Q1240.0 -208 1240.0 -298Q1240.0 -373 1263.0 -435.0Q1286.0 -497 1337.5 -544.5Q1389.0 -592 1476.0 -624Q1536.0 -646 1619.0 -663.0Q1702.0 -680 1807.0 -695.5Q1912.0 -711 2038.0 -730L1940.0 -676Q1940.0 -772 1894.0 -817.0Q1848.0 -862 1740.0 -862Q1680.0 -862 1615.0 -833.0Q1550.0 -804 1524.0 -730L1278.0 -808Q1319.0 -942 1432.0 -1026.0Q1545.0 -1110 1740.0 -1110Q1883.0 -1110 1994.0 -1066.0Q2105.0 -1022 2162.0 -914Q2194.0 -854 2200.0 -794.0Q2206.0 -734 2206.0 -660V0H1968.0V-222L2002.0 -176Q1923.0 -67 1831.5 -18.5Q1740.0 30 1600.0 30ZM1658.0 -184Q1733.0 -184 1784.5 -210.5Q1836.0 -237 1866.5 -271.0Q1897.0 -305 1908.0 -328Q1929.0 -372 1932.5 -430.5Q1936.0 -489 1936.0 -528L2016.0 -508Q1895.0 -488 1820.0 -474.5Q1745.0 -461 1699.0 -450.0Q1653.0 -439 1618.0 -426Q1578.0 -410 1553.5 -391.5Q1529.0 -373 1517.5 -351.0Q1506.0 -329 1506.0 -302Q1506.0 -265 1524.5 -238.5Q1543.0 -212 1577.0 -198.0Q1611.0 -184 1658.0 -184ZM2936.0 30Q2775.0 30 2667.0 -45.0Q2559.0 -120 2504.5 -249.0Q2450.0 -378 2450.0 -540Q2450.0 -702 2504.0 -831.0Q2558.0 -960 2663.0 -1035.0Q2768.0 -1110 2922.0 -1110Q3077.0 -1110 3192.0 -1036.0Q3307.0 -962 3370.5 -833.5Q3434.0 -705 3434.0 -540Q3434.0 -378 3371.5 -249.0Q3309.0 -120 3197.0 -45.0Q3085.0 30 2936.0 30ZM2404.0 0V-1440H2678.0V-740H2644.0V0ZM2892.0 -212Q2980.0 -212 3036.0 -256.0Q3092.0 -300 3119.0 -374.5Q3146.0 -449 3146.0 -540Q3146.0 -630 3118.0 -704.5Q3090.0 -779 3031.5 -823.5Q2973.0 -868 2882.0 -868Q2797.0 -868 2744.5 -827.0Q2692.0 -786 2668.0 -712.0Q2644.0 -638 2644.0 -540Q2644.0 -442 2668.0 -368.0Q2692.0 -294 2746.5 -253.0Q2801.0 -212 2892.0 -212Z', GW = 'M3896.0 0 3474.0 -1440H3760.0L4030.0 -450L4300.0 -1438L4586.0 -1440L4856.0 -450L5126.0 -1440H5412.0L4990.0 0H4722.0L4443.0 -972L4164.0 0ZM5812.0 30Q5696.0 30 5615.5 -14.5Q5535.0 -59 5493.5 -133.5Q5452.0 -208 5452.0 -298Q5452.0 -373 5475.0 -435.0Q5498.0 -497 5549.5 -544.5Q5601.0 -592 5688.0 -624Q5748.0 -646 5831.0 -663.0Q5914.0 -680 6019.0 -695.5Q6124.0 -711 6250.0 -730L6152.0 -676Q6152.0 -772 6106.0 -817.0Q6060.0 -862 5952.0 -862Q5892.0 -862 5827.0 -833.0Q5762.0 -804 5736.0 -730L5490.0 -808Q5531.0 -942 5644.0 -1026.0Q5757.0 -1110 5952.0 -1110Q6095.0 -1110 6206.0 -1066.0Q6317.0 -1022 6374.0 -914Q6406.0 -854 6412.0 -794.0Q6418.0 -734 6418.0 -660V0H6180.0V-222L6214.0 -176Q6135.0 -67 6043.5 -18.5Q5952.0 30 5812.0 30ZM5870.0 -184Q5945.0 -184 5996.5 -210.5Q6048.0 -237 6078.5 -271.0Q6109.0 -305 6120.0 -328Q6141.0 -372 6144.5 -430.5Q6148.0 -489 6148.0 -528L6228.0 -508Q6107.0 -488 6032.0 -474.5Q5957.0 -461 5911.0 -450.0Q5865.0 -439 5830.0 -426Q5790.0 -410 5765.5 -391.5Q5741.0 -373 5729.5 -351.0Q5718.0 -329 5718.0 -302Q5718.0 -265 5736.5 -238.5Q5755.0 -212 5789.0 -198.0Q5823.0 -184 5870.0 -184ZM6638.0 0V-1470H6910.0V0ZM7170.0 0V-1470H7442.0V0ZM8184.0 30Q8018.0 30 7891.5 -41.5Q7765.0 -113 7693.5 -238.5Q7622.0 -364 7622.0 -526Q7622.0 -703 7692.0 -834.0Q7762.0 -965 7885.0 -1037.5Q8008.0 -1110 8168.0 -1110Q8338.0 -1110 8457.0 -1030.0Q8576.0 -950 8633.0 -805.0Q8690.0 -660 8673.0 -464H8404.0V-564Q8404.0 -729 8351.5 -801.5Q8299.0 -874 8180.0 -874Q8041.0 -874 7975.5 -789.5Q7910.0 -705 7910.0 -540Q7910.0 -389 7975.5 -306.5Q8041.0 -224 8168.0 -224Q8248.0 -224 8305.0 -259.0Q8362.0 -294 8392.0 -360L8664.0 -282Q8603.0 -134 8471.5 -52.0Q8340.0 30 8184.0 30ZM7826.0 -464V-666H8542.0V-464ZM9486.0 0Q9374.0 21 9266.5 18.5Q9159.0 16 9074.5 -19.5Q8990.0 -55 8946.0 -134Q8906.0 -208 8904.0 -284.5Q8902.0 -361 8902.0 -458V-1380H9174.0V-474Q9174.0 -411 9175.5 -360.5Q9177.0 -310 9196.0 -280Q9232.0 -223 9311.0 -218.0Q9390.0 -213 9486.0 -226ZM8718.0 -870V-1080H9486.0V-870Z';
  function Logo(p) {
    var variant = p.variant || 'full', height = p.height || 32, tone = p.tone || 'default';
    var vbw = variant === 'mark' ? 64 : LOGO_W;
    var kids = [
      h('rect', { key: 't', className: 'tw-logo-tab', x: 8, y: 8, width: 26, height: 16, rx: 6 }),
      h('rect', { key: 'b', className: 'tw-logo-body', x: 4, y: 16, width: 56, height: 42, rx: 11 }),
      h('rect', { key: 'c', className: 'tw-logo-clasp', x: 38, y: 30, width: 22, height: 14, rx: 7 }),
      h('circle', { key: 'd', className: 'tw-logo-dot', cx: 46, cy: 37, r: 3 })
    ];
    if (variant !== 'mark') kids.push(h('g', { key: 'w', transform: WT },
      h('path', { className: 'tw-logo-w1', d: GT }), h('path', { className: 'tw-logo-w2', d: GW })));
    return h('svg', {
      className: cx('tw-logo', 'tw-logo-' + tone, p.className), viewBox: '0 0 ' + vbw + ' 64',
      height: height, width: Math.round(height * vbw / 64), role: 'img', 'aria-label': p.label || 'TabWallet'
    }, kids);
  }

  /* ---------- Button ---------- */
  function Button(p) {
    var variant = p.variant || 'primary', size = p.size || 'md';
    var iconOnly = !p.children && (p.icon || p.iconRight);
    var isz = size === 'sm' ? 16 : 20;
    var rest = omit(p, ['variant', 'size', 'icon', 'iconRight', 'fullWidth', 'loading', 'className', 'children', 'type']);
    return h('button', Object.assign({
      type: p.type || 'button',
      className: cx('tw-btn', 'tw-btn-' + variant, 'tw-btn-' + size, iconOnly && 'tw-btn-icon', p.fullWidth && 'tw-btn-full', p.className),
      'aria-busy': p.loading ? true : undefined, disabled: p.disabled || p.loading
    }, rest),
      p.loading ? h('span', { className: 'tw-spinner', 'aria-hidden': true }) : (p.icon ? h(Icon, { name: p.icon, size: isz }) : null),
      p.children ? h('span', null, p.children) : null,
      p.iconRight ? h(Icon, { name: p.iconRight, size: isz }) : null);
  }

  /* ---------- SegmentedControl ---------- */
  function SegmentedControl(p) {
    var opts = p.options || [];
    var st = React.useState(p.defaultValue != null ? p.defaultValue : (opts[0] && opts[0].value));
    var value = p.value != null ? p.value : st[0];
    function pick(v) { if (p.value == null) st[1](v); if (p.onChange) p.onChange(v); }
    return h('div', { className: cx('tw-seg', p.size === 'sm' && 'tw-seg-sm', p.className), role: 'radiogroup', 'aria-label': p.label },
      opts.map(function (o) {
        var on = o.value === value;
        return h('button', {
          key: o.value, type: 'button', role: 'radio', 'aria-checked': on, tabIndex: on ? 0 : -1,
          className: 'tw-seg-item', onClick: function () { pick(o.value); },
          onKeyDown: function (e) {
            var i = opts.indexOf(o), d = e.key === 'ArrowRight' ? 1 : e.key === 'ArrowLeft' ? -1 : 0;
            if (d) { e.preventDefault(); var n = opts[(i + d + opts.length) % opts.length]; pick(n.value);
              var btns = e.currentTarget.parentNode.querySelectorAll('.tw-seg-item'); btns[opts.indexOf(n)].focus(); }
          }
        }, o.label);
      }));
  }

  /* ---------- Field helpers ---------- */
  function Field(p, control) {
    return h('div', { className: cx('tw-field', p.className) },
      p.label ? h('label', { className: 'tw-label', htmlFor: p.id }, p.label, p.optional ? h('span', { className: 'tw-optional' }, ' (opcional)') : null) : null,
      control,
      p.error ? h('p', { className: 'tw-hint tw-error', id: p.id + '-msg' }, h(Icon, { name: 'circle-alert', size: 16 }), h('span', null, p.error))
        : p.hint ? h('p', { className: 'tw-hint', id: p.id + '-msg' }, p.hint) : null);
  }
  var FIELD_KEYS = ['label', 'hint', 'error', 'optional', 'className', 'id', 'options', 'placeholderOption'];
  function aria(p, id) { return { id: id, 'aria-invalid': p.error ? true : undefined, 'aria-describedby': (p.error || p.hint) ? id + '-msg' : undefined }; }

  function TextField(p) {
    var gid = useId(), id = p.id || gid;
    var input = h('input', Object.assign({ type: 'text', className: 'tw-input' }, omit(p, FIELD_KEYS), aria(p, id)));
    return Field(Object.assign({}, p, { id: id }), input);
  }

  function MoneyInput(p) {
    var gid = useId(), id = p.id || gid;
    var input = h('div', { className: 'tw-input-group' },
      h('span', { className: 'tw-input-prefix', 'aria-hidden': true }, 'R$'),
      h('input', Object.assign({ type: 'text', inputMode: 'decimal', placeholder: '0,00', autoComplete: 'off', className: 'tw-input tw-input-money' }, omit(p, FIELD_KEYS), aria(p, id))));
    return Field(Object.assign({}, p, { id: id }), input);
  }

  function Select(p) {
    var gid = useId(), id = p.id || gid;
    var opts = (p.options || []).map(function (o) { return h('option', { key: o.value, value: o.value, disabled: o.disabled }, o.label); });
    if (p.placeholderOption) opts.unshift(h('option', { key: '__ph', value: '', disabled: true }, p.placeholderOption));
    var sel = h('div', { className: 'tw-select-wrap' },
      h('select', Object.assign({ className: 'tw-input tw-select' }, omit(p, FIELD_KEYS), aria(p, id)), opts),
      h(Icon, { name: 'chevron-down', size: 20, className: 'tw-select-chevron' }));
    return Field(Object.assign({}, p, { id: id }), sel);
  }

  /* ---------- Money ---------- */
  function Money(p) {
    var r = signedBRL(p.value, p.showSign === false ? 'neutral' : (p.kind || 'auto'));
    var kind = p.showSign === false ? (p.kind || 'neutral') : r.kind;
    if (p.showSign === false) r.text = (Number(p.value) < 0 ? MINUS + NBSP : '') + formatBRL(p.value);
    return h('span', { className: cx('tw-money', 'tw-money-' + (p.size || 'md'), 'tw-money-' + kind, p.className) }, r.text);
  }

  /* ---------- Badge ---------- */
  function Badge(p) {
    return h('span', { className: cx('tw-badge', 'tw-badge-' + (p.tone || 'neutral'), p.className) },
      p.icon ? h(Icon, { name: p.icon, size: 14 }) : null, p.children);
  }

  /* ---------- TransactionRow ---------- */
  function TransactionRow(p) {
    var n = Number(p.amount) || 0;
    var kind = p.kind || (n >= 0 ? 'income' : 'expense');
    var meta = [];
    if (p.category) meta.push(h(Badge, { key: 'c', icon: 'tag' }, p.category));
    if (p.date) meta.push(h('span', { key: 'd' }, p.date));
    if (p.installment) meta.push(h(Badge, { key: 'i', icon: 'credit-card' }, 'Parcela ' + p.installment.current + '/' + p.installment.total));
    if (p.recurring) meta.push(h(Badge, { key: 'r', icon: 'repeat' }, 'Fixo'));
    var El = p.as || 'li';
    return h(El, { className: cx('tw-txn', p.className) },
      h('span', { className: 'tw-txn-icon tw-txn-icon-' + kind },
        h(Icon, { name: kind === 'income' ? 'arrow-up-right' : 'arrow-down-left', size: 20, label: kind === 'income' ? 'Receita' : 'Despesa' })),
      h('div', { className: 'tw-txn-main' },
        h('p', { className: 'tw-txn-desc' }, p.description),
        meta.length ? h('div', { className: 'tw-txn-meta' }, meta) : null),
      h(Money, { value: Math.abs(n), kind: kind, className: 'tw-txn-amount' }),
      (p.onEdit || p.onDelete) ? h('div', { className: 'tw-txn-actions' },
        p.onEdit ? h(Button, { variant: 'ghost', size: 'sm', icon: 'pencil', 'aria-label': 'Editar ' + p.description, onClick: p.onEdit }) : null,
        p.onDelete ? h(Button, { variant: 'ghost', size: 'sm', icon: 'trash-2', 'aria-label': 'Excluir ' + p.description, onClick: p.onDelete }) : null) : null);
  }

  /* ---------- Card ---------- */
  function Card(p) {
    var El = p.as || 'section';
    return h(El, { className: cx('tw-card', p.flush && 'tw-card-flush', p.className) },
      (p.title || p.action) ? h('header', { className: 'tw-card-head' },
        h('div', null, p.title ? h('h3', { className: 'tw-card-title' }, p.title) : null,
          p.subtitle ? h('p', { className: 'tw-card-sub' }, p.subtitle) : null),
        p.action || null) : null,
      p.children);
  }

  /* ---------- Dialog ---------- */
  function Dialog(p) {
    var tid = useId();
    if (p.open === false) return null;
    return h('div', { className: cx('tw-dialog-backdrop', p.inline && 'tw-dialog-inline'), onClick: function (e) { if (e.target === e.currentTarget && p.onClose) p.onClose(); },
      onKeyDown: function (e) { if (e.key === 'Escape' && p.onClose) p.onClose(); } },
      h('div', { className: 'tw-dialog', role: p.tone === 'danger' ? 'alertdialog' : 'dialog', 'aria-modal': true, 'aria-labelledby': tid },
        h('h2', { className: 'tw-dialog-title', id: tid }, p.title),
        p.children ? h('div', { className: 'tw-dialog-body' }, p.children) : null,
        p.actions ? h('div', { className: 'tw-dialog-actions' }, p.actions) : null));
  }

  /* ---------- EmptyState ---------- */
  function EmptyState(p) {
    return h('div', { className: cx('tw-empty', p.className) },
      h('span', { className: 'tw-empty-icon' }, h(Icon, { name: p.icon || 'inbox', size: 24 })),
      h('h3', { className: 'tw-empty-title' }, p.title),
      p.description ? h('p', { className: 'tw-empty-desc' }, p.description) : null,
      p.action || null);
  }

  /* ---------- Alert ---------- */
  var ALERT_ICON = { info: 'info', success: 'circle-check', warning: 'triangle-alert', danger: 'circle-alert' };
  function Alert(p) {
    var tone = p.tone || 'info';
    return h('div', { className: cx('tw-alert', 'tw-alert-' + tone, p.className), role: (tone === 'danger' || tone === 'warning') ? 'alert' : 'status' },
      h(Icon, { name: ALERT_ICON[tone], size: 20, className: 'tw-alert-icon' }),
      h('div', { className: 'tw-alert-main' },
        p.title ? h('p', { className: 'tw-alert-title' }, p.title) : null,
        p.children ? h('div', { className: 'tw-alert-body' }, p.children) : null),
      p.action || null);
  }

  /* ---------- BalanceCard ---------- */
  function BalanceCard(p) {
    function flow(kind, label, v) {
      return h('div', { className: 'tw-flow' },
        h('span', { className: 'tw-flow-icon tw-txn-icon-' + kind }, h(Icon, { name: kind === 'income' ? 'arrow-up-right' : 'arrow-down-left', size: 16 })),
        h('div', null, h('p', { className: 'tw-flow-label' }, label), h(Money, { value: v, kind: kind })));
    }
    return h('section', { className: cx('tw-card tw-balance', p.className) },
      h('div', { className: 'tw-balance-top' },
        h('p', { className: 'tw-balance-label' }, p.label || 'Saldo atual'),
        p.period ? h('p', { className: 'tw-balance-period' }, p.period) : null),
      h(Money, { value: p.balance, showSign: false, kind: Number(p.balance) < 0 ? 'expense' : 'neutral', size: 'lg' }),
      h('div', { className: 'tw-balance-flows' }, flow('income', 'Receitas', Math.abs(p.income || 0)), flow('expense', 'Despesas', Math.abs(p.expense || 0))));
  }

  /* ---------- BudgetBar ---------- */
  function BudgetBar(p) {
    var spent = Math.abs(Number(p.spent) || 0), limit = Math.abs(Number(p.limit) || 0);
    var pct = limit ? spent / limit : 0, shown = Math.round(pct * 100);
    var warnAt = p.warnAt || 0.8;
    var state = pct > 1 ? 'over' : pct >= warnAt ? 'near' : 'ok';
    var status = state === 'over' ? 'Limite excedido em ' + formatBRL(spent - limit) : state === 'near' ? shown + '% do limite' : shown + '% usado';
    return h('div', { className: cx('tw-budget', 'tw-budget-' + state, p.className) },
      h('div', { className: 'tw-budget-head' },
        h('p', { className: 'tw-budget-name' }, p.category),
        h('p', { className: 'tw-budget-status' }, state !== 'ok' ? h(Icon, { name: 'triangle-alert', size: 16 }) : null, status)),
      h('div', { className: 'tw-budget-track', role: 'progressbar', 'aria-label': 'Orçamento de ' + p.category, 'aria-valuemin': 0, 'aria-valuemax': 100, 'aria-valuenow': Math.min(shown, 100), 'aria-valuetext': status },
        h('div', { className: 'tw-budget-fill', style: { width: Math.min(pct, 1) * 100 + '%' } })),
      h('p', { className: 'tw-budget-foot' }, formatBRL(spent) + ' de ' + formatBRL(limit)));
  }

  /* ---------- InvoiceCard ---------- */
  var INV = { open: ['Aberta', 'brand'], closed: ['Fechada', 'warning'], paid: ['Paga', 'income'] };
  function InvoiceCard(p) {
    var st = INV[p.status || 'open'];
    var total = Math.abs(Number(p.total) || 0), paid = Math.abs(Number(p.paid) || 0);
    var meta = [];
    if (p.closingDay) meta.push('Fecha dia ' + p.closingDay);
    if (p.dueDate) meta.push('vence em ' + p.dueDate);
    return h('section', { className: cx('tw-card tw-invoice', p.className) },
      h('div', { className: 'tw-invoice-head' },
        h('span', { className: 'tw-invoice-icon' }, h(Icon, { name: 'credit-card', size: 20 })),
        h('div', { className: 'tw-invoice-title' }, h('h3', { className: 'tw-card-title' }, p.cardName), p.month ? h('p', { className: 'tw-card-sub' }, 'Fatura de ' + p.month) : null),
        h(Badge, { tone: st[1] }, st[0])),
      h(Money, { value: total, showSign: false, size: 'lg' }),
      meta.length ? h('p', { className: 'tw-invoice-meta' }, meta.join(' · ')) : null,
      (paid > 0 && p.status !== 'paid') ? h('p', { className: 'tw-invoice-meta' }, 'Adiantado: ' + formatBRL(paid) + ' · Restante: ' + formatBRL(Math.max(total - paid, 0))) : null,
      (p.status !== 'paid' && p.onAdvance) ? h(Button, { variant: 'secondary', icon: 'wallet', onClick: p.onAdvance }, 'Adiantar pagamento') : null);
  }

  /* ---------- AppHeader ---------- */
  function AppHeader(p) {
    var items = p.items || [];
    return h('header', { className: cx('tw-header', p.className) },
      h('a', { className: 'tw-header-brand', href: p.homeHref || '#', 'aria-label': 'TabWallet — início' }, h(Logo, { height: 28 })),
      items.length ? h('nav', { className: 'tw-nav', 'aria-label': 'Principal' },
        items.map(function (it) {
          return h('a', { key: it.label, href: it.href || '#', className: 'tw-nav-link', 'aria-current': it.active ? 'page' : undefined },
            it.icon ? h(Icon, { name: it.icon, size: 20 }) : null, h('span', { className: 'tw-nav-label' }, it.label));
        })) : null,
      p.right ? h('div', { className: 'tw-header-right' }, p.right) : null);
  }

  var api = { Logo: Logo, Icon: Icon, Button: Button, SegmentedControl: SegmentedControl, TextField: TextField, MoneyInput: MoneyInput,
    Select: Select, Money: Money, Badge: Badge, TransactionRow: TransactionRow, Card: Card, Dialog: Dialog, EmptyState: EmptyState,
    Alert: Alert, BalanceCard: BalanceCard, BudgetBar: BudgetBar, InvoiceCard: InvoiceCard, AppHeader: AppHeader,
    formatBRL: formatBRL, signedBRL: signedBRL };
  window.TabWallet = Object.assign(window.TabWallet || {}, api);
})();
