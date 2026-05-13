

import onnx
import onnx2torch

onnx_model = onnx.load('../spike/models/en_pp-ocrv5_mobile_rec.onnx')

torch_model = onnx2torch.convert(onnx_model).eval()

print('PyTorch model structure:')
lstm_count = 0
rnn_count = 0
gru_count = 0
for name, module in torch_model.named_modules():
    if hasattr(module, '__class__'):
        module_type = module.__class__.__name__
        if 'LSTM' in module_type:
            print(f'  LSTM: {name}: {module_type}')
            lstm_count += 1
        elif 'RNN' in module_type:
            print(f'  RNN: {name}: {module_type}')
            rnn_count += 1
        elif 'GRU' in module_type:
            print(f'  GRU: {name}: {module_type}')
            gru_count += 1

print(f'\nSummary: LSTM={lstm_count}, RNN={rnn_count}, GRU={gru_count}')
