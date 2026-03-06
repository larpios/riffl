import re

with open('src/export.rs', 'r') as f:
    content = f.read()

# We need to change `samples: &[Sample]` to `samples: &[Arc<Sample>]` in the function signature
content = content.replace('samples: &[Sample],', 'samples: &[Arc<Sample>],')

# Then we need to make sure the call sites inside src/export.rs tests are providing &[Arc<Sample>]
content = content.replace('&[sample],', '&[Arc::new(sample)],')
content = content.replace('&[sample.clone()],', '&[Arc::new(sample.clone())],')

# Handle edge cases not caught by the comma
content = content.replace('&[sample]', '&[Arc::new(sample)]')
content = content.replace('&[sample.clone()]', '&[Arc::new(sample.clone())]')

# However, we must ensure we import Arc at the top of src/export.rs
if 'use std::sync::Arc;' not in content:
    content = content.replace('use std::path::Path;', 'use std::path::Path;\nuse std::sync::Arc;')

with open('src/export.rs', 'w') as f:
    f.write(content)
