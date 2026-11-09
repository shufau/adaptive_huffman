use wasm_bindgen::prelude::*;

const NYT_SYMBOL: i32 = -1;
const INTERNAL_SYMBOL: i32 = -2;

/// 節點結構（改用陣列索引代替 Box 指標）
#[derive(Clone, Copy)]
struct Node {
    weight: usize,
    parent: usize,
    left: usize,
    right: usize,
    symbol: i32,
}

/// 動態霍夫曼樹 (FGK 演算法實作)
struct AdaptiveHuffman {
    nodes: Vec<Node>,
    nyt_index: usize,
    root_index: usize,
    map: [usize; 256], // 紀錄 ASCII 字元對應的葉節點索引
}

impl AdaptiveHuffman {
    fn new() -> Self {
        // 分配 514 個節點。索引 0 作為 null 指標，1~513 作為有效節點
        // 256 個字元最多會產生 256 個葉節點 + 255 個內部節點 = 511 個節點
        let mut nodes = vec![Node { weight: 0, parent: 0, left: 0, right: 0, symbol: 0 }; 514];
        
        // 初始狀態下，根節點就是 NYT 節點
        nodes[513] = Node {
            weight: 0,
            parent: 0,
            left: 0,
            right: 0,
            symbol: NYT_SYMBOL,
        };
        
        AdaptiveHuffman {
            nodes,
            nyt_index: 513,
            root_index: 513,
            map: [0; 256],
        }
    }

    /// 交換兩個節點以維持霍夫曼樹的兄弟屬性（已修復指標環路 Bug）
    fn swap_nodes(&mut self, a: usize, b: usize) {
        let pa = self.nodes[a].parent;
        let pb = self.nodes[b].parent;
        
        // 1. 直接交換陣列中兩節點的資料內容
        self.nodes.swap(a, b);
        
        // 2. 恢復兩陣列位置對應的父節點指標 (位置 a 的父節點始終是 pa，位置 b 是 pb)
        self.nodes[a].parent = pa;
        self.nodes[b].parent = pb;
        
        // 3. 更新被移到位置 a 的節點，其子節點的父指標
        let a_left = self.nodes[a].left;
        if a_left != 0 { self.nodes[a_left].parent = a; }
        let a_right = self.nodes[a].right;
        if a_right != 0 { self.nodes[a_right].parent = a; }
        
        // 4. 更新被移到位置 b 的節點，其子節點的父指標
        let b_left = self.nodes[b].left;
        if b_left != 0 { self.nodes[b_left].parent = b; }
        let b_right = self.nodes[b].right;
        if b_right != 0 { self.nodes[b_right].parent = b; }
        
        // 5. 更新字元映射表與 NYT 索引
        if self.nodes[a].symbol >= 0 { self.map[self.nodes[a].symbol as usize] = a; }
        if self.nodes[b].symbol >= 0 { self.map[self.nodes[b].symbol as usize] = b; }
        if self.nodes[a].symbol == NYT_SYMBOL { self.nyt_index = a; }
        if self.nodes[b].symbol == NYT_SYMBOL { self.nyt_index = b; }
    }

    /// 更新權重並重新平衡樹結構
    fn update(&mut self, mut node: usize) {
        while node != 0 { // 直到走到根節點之上 (null)
            let mut max_idx = node;
            
            // 找出相同權重中，隱含編號最大（也就是索引最大）的節點
            for i in (node + 1)..=513 {
                if self.nodes[i].weight == self.nodes[node].weight {
                    max_idx = i;
                } else if self.nodes[i].weight > self.nodes[node].weight {
                    break; // 陣列維持按權重排序，提早結束
                }
            }
            
            // 如果找到的節點不是自己，也不是自己的父節點，就進行交換
            if max_idx != node && max_idx != self.nodes[node].parent {
                self.swap_nodes(node, max_idx);
                node = max_idx;
            }
            
            self.nodes[node].weight += 1;
            node = self.nodes[node].parent;
        }
    }

    /// 加入新的字元
    fn add_symbol(&mut self, symbol: u8) {
        let nyt = self.nyt_index;
        let left = nyt - 2; // 新的 NYT
        let right = nyt - 1; // 新的字元葉節點
        
        // 設定新的 NYT
        self.nodes[left].symbol = NYT_SYMBOL;
        self.nodes[left].weight = 0;
        self.nodes[left].parent = nyt;
        self.nodes[left].left = 0;
        self.nodes[left].right = 0;
        
        // 設定新的字元葉節點
        self.nodes[right].symbol = symbol as i32;
        self.nodes[right].weight = 0;
        self.nodes[right].parent = nyt;
        self.nodes[right].left = 0;
        self.nodes[right].right = 0;
        
        // 舊的 NYT 變成內部節點
        self.nodes[nyt].symbol = INTERNAL_SYMBOL;
        self.nodes[nyt].left = left;
        self.nodes[nyt].right = right;
        
        self.nyt_index = left;
        self.map[symbol as usize] = right;
        
        // 從新的字元葉節點開始向上更新權重
        self.update(right);
    }

    /// 取得從根節點到目標節點的路徑
    fn get_path(&self, mut node: usize) -> Vec<u8> {
        let mut path = Vec::new();
        while self.nodes[node].parent != 0 {
            let p = self.nodes[node].parent;
            if self.nodes[p].left == node {
                path.push(0);
            } else {
                path.push(1);
            }
            node = p;
        }
        path.reverse(); // 因為是從下往上找，最後需要反轉
        path
    }
}

// ---------------- 以下為位元操作與外部接口 ----------------

struct BitWriter {
    bytes: Vec<u8>,
    current_byte: u8,
    bit_count: u8,
}

impl BitWriter {
    fn new() -> Self {
        BitWriter { bytes: Vec::new(), current_byte: 0, bit_count: 0 }
    }

    fn write_bit(&mut self, bit: u8) {
        if bit != 0 { self.current_byte |= 1 << (7 - self.bit_count); }
        self.bit_count += 1;
        if self.bit_count == 8 {
            self.bytes.push(self.current_byte);
            self.current_byte = 0;
            self.bit_count = 0;
        }
    }

    fn write_path(&mut self, path: &[u8]) {
        for &bit in path { self.write_bit(bit); }
    }

    fn write_bits(&mut self, value: u32, count: u8) {
        for i in (0..count).rev() {
            let bit = ((value >> i) & 1) as u8;
            self.write_bit(bit);
        }
    }

    fn flush(mut self) -> Vec<u8> {
        if self.bit_count > 0 { self.bytes.push(self.current_byte); }
        self.bytes
    }
}

struct BitReader<'a> {
    data: &'a [u8],
    byte_index: usize,
    bit_index: u8,
}

impl<'a> BitReader<'a> {
    fn new(data: &'a [u8]) -> Self {
        BitReader { data, byte_index: 0, bit_index: 0 }
    }

    fn read_bit(&mut self) -> Result<u8, &'static str> {
        if self.byte_index >= self.data.len() { return Err("Unexpected end of bit stream"); }
        let bit = (self.data[self.byte_index] >> (7 - self.bit_index)) & 1;
        self.bit_index += 1;
        if self.bit_index == 8 {
            self.bit_index = 0;
            self.byte_index += 1;
        }
        Ok(bit)
    }

    fn read_bits(&mut self, count: u8) -> Result<u32, &'static str> {
        let mut value = 0;
        for _ in 0..count {
            value = (value << 1) | (self.read_bit()? as u32);
        }
        Ok(value)
    }
}

fn core_compress(input: &[u8]) -> Vec<u8> {
    let mut writer = BitWriter::new();
    let mut tree = AdaptiveHuffman::new();

    for &byte in input {
        let node_idx = tree.map[byte as usize];
        if node_idx != 0 {
            // 字元已存在，輸出路徑並更新樹
            let path = tree.get_path(node_idx);
            writer.write_path(&path);
            tree.update(node_idx);
        } else {
            // 新字元，輸出 NYT 路徑 -> 原字元 -> 擴展樹
            let path = tree.get_path(tree.nyt_index);
            writer.write_path(&path);
            writer.write_bits(byte as u32, 8);
            tree.add_symbol(byte);
        }
    }
    writer.flush()
}

fn core_decompress(input: &[u8], original_len: usize) -> Result<Vec<u8>, &'static str> {
    let mut reader = BitReader::new(input);
    let mut output = Vec::with_capacity(original_len);

    if original_len == 0 { return Ok(output); }

    let mut tree = AdaptiveHuffman::new();

    // 處理第一個原始字元
    let first_byte = reader.read_bits(8)? as u8;
    output.push(first_byte);
    tree.add_symbol(first_byte);

    while output.len() < original_len {
        let mut curr = tree.root_index;
        
        // 往下走訪樹，直到抵達葉節點
        while tree.nodes[curr].left != 0 && tree.nodes[curr].right != 0 {
            let bit = reader.read_bit()?;
            if bit == 0 { curr = tree.nodes[curr].left; }
            else { curr = tree.nodes[curr].right; }
        }

        if tree.nodes[curr].symbol == NYT_SYMBOL {
            // 抵達 NYT 節點，讀取接下來的 8 bits
            let new_byte = reader.read_bits(8)? as u8;
            output.push(new_byte);
            tree.add_symbol(new_byte);
        } else {
            // 抵達已知字元節點
            let sym = tree.nodes[curr].symbol as u8;
            output.push(sym);
            tree.update(curr);
        }
    }

    Ok(output)
}

#[wasm_bindgen]
pub fn compress_data(input: &[u8]) -> Vec<u8> {
    if input.is_empty() { return vec![0x00]; }

    let compressed_body = core_compress(input);
    let orig_len = input.len() as u32;
    let mut result = Vec::with_capacity(1 + 4 + compressed_body.len());
    
    if compressed_body.len() >= input.len() {
        result.push(0x00);
        result.extend_from_slice(&orig_len.to_le_bytes());
        result.extend_from_slice(input);
    } else {
        result.push(0x01);
        result.extend_from_slice(&orig_len.to_le_bytes());
        result.extend_from_slice(&compressed_body);
    }
    result
}

#[wasm_bindgen]
pub fn decompress_data(input: &[u8]) -> Vec<u8> {
    if input.len() < 5 { return Vec::new(); }

    let flag = input[0];
    let orig_len = u32::from_le_bytes([input[1], input[2], input[3], input[4]]) as usize;
    let payload = &input[5..];

    if flag == 0x00 {
        return payload[..orig_len.min(payload.len())].to_vec();
    }

    match core_decompress(payload, orig_len) {
        Ok(res) => res,
        Err(_) => payload.to_vec(),
    }
}